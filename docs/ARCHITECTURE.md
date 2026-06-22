# FAF Co-op Deployer — Architecture

> **Status:** Phase 0 (planning). No implementation yet.
> **Stack:** Rust HTTP service (axum + tokio), PostgreSQL (sqlx), OAuth2 (FAF Hydra JWT).
> **Prime directive:** Services never touch IO directly. Ports are the only door to the outside world.
> Everything testable without a database, network, or filesystem.

This document is the contract for how the service is built. If a change violates a rule here,
change the rule on purpose — don't quietly break it.

---

## 1. What problem does this solve?

Co-op map and patch deployment at FAForever is entirely manual today:

- Two people (Brutus, Sheikah) run Kotlin scripts by hand
- The scripts talk directly to the production database
- There is no interface for the campaign team to self-serve
- Campaigns (grouping missions into a series) have no backend support at all

This service replaces the scripts with a proper HTTP API. Campaign team members with the
`COOP_DEPLOYER` role can deploy maps, deploy patches, and manage campaigns themselves —
without database access, without shell access, without involving Brutus or Sheikah.

---

## 2. Core principle — ports isolate all IO

Every external system (database, filesystem, Git, GitHub API, FAF Hydra) is hidden behind a
`Port` trait. Services call ports. Infra implements ports. The two never meet directly.

```
HTTP Request
    │
    ▼
Middleware (auth)          ← verifies Bearer token via AuthPort
    │
    ▼
Handler                    ← thin: parse request, call service, serialize response
    │
    ▼
Service                    ← business logic only. calls ports, returns results. no IO.
    │           │
    ▼           ▼
Port A      Port B         ← trait definitions. the only thing services see.
    │           │
    ▼           ▼
Infra A     Infra B        ← real IO (DB queries, HTTP calls, file writes).
                              in tests: Fake A / Fake B (in-memory, no network).
```

### Non-negotiable rules

1. **Services never import `sqlx`, `reqwest`, `git2`, or any IO crate.** Only port traits.
2. **Infra is the only place with real IO.** Everything else is pure logic.
3. **Every port has a Fake.** If you add a real infra impl, you also add a fake for tests.
4. **Handlers contain no business logic.** Parse → call service → serialize. Nothing more.
5. **`DRY_RUN=true` must work** at any point. Fakes make this free.

---

## 3. Crate structure

```
faf-coop-deployer/
├── Cargo.toml                  # workspace
├── src/
│   └── main.rs                 # entry point: build Ports, start axum server
├── crates/
│   ├── coop-domain/            # PURE. Zero IO, no async, no tokio.
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models.rs       # all domain types
│   │       └── errors.rs       # all error types
│   │
│   └── coop-app/               # ALL async + IO lives here.
│       └── src/
│           ├── lib.rs
│           ├── ports/          # trait definitions — what services are allowed to call
│           │   ├── mod.rs      # Ports bundle
│           │   ├── auth.rs     # AuthPort
│           │   ├── db.rs       # DbPort
│           │   ├── git.rs      # GitPort
│           │   ├── fs.rs       # FsPort
│           │   └── github.rs   # GithubPort
│           ├── infra/          # concrete implementations — the ONLY IO-permitted zone
│           │   ├── mod.rs      # real_ports() / fake_ports() / ports_from_env()
│           │   ├── auth.rs     # HydraAuth (JWT verify via JWKS)
│           │   ├── auth_fake.rs
│           │   ├── db.rs       # SqlxDb (PostgreSQL)
│           │   ├── db_fake.rs
│           │   ├── git.rs      # GitInfra (git2)
│           │   ├── git_fake.rs
│           │   ├── fs.rs       # LocalFs
│           │   ├── fs_fake.rs
│           │   ├── github.rs   # GithubClient (reqwest)
│           │   └── github_fake.rs
│           ├── services/       # business logic, one module per domain
│           │   ├── mod.rs
│           │   ├── auth.rs     # token verify + role check
│           │   ├── map.rs      # map deploy (ported from CoopMapDeployer.kt)
│           │   ├── patch.rs    # patch deploy (ported from CoopDeployer.kt)
│           │   └── campaign.rs # campaign management (new — no script existed)
│           └── http/           # axum layer
│               ├── mod.rs      # router
│               ├── middleware.rs
│               └── handlers/
│                   ├── maps.rs
│                   ├── patches.rs
│                   └── campaigns.rs
├── tests/                      # integration tests (FakePorts, no network)
│   ├── map_deploy.rs
│   ├── patch_deploy.rs
│   └── campaign.rs
└── docs/
    ├── ARCHITECTURE.md         # this document
    └── PLAN.md                 # phase-by-phase build plan
```

### Dependency direction (enforced by the crate graph)

```
coop-domain  ← depends on NOTHING
coop-app     ← depends on coop-domain
src/main.rs  ← depends on coop-app
```

A service can never reach a socket or the filesystem directly. `infra` is the only module
allowed to do real IO. This is what makes everything testable without a running database.

---

## 4. Domain types (coop-domain)

All types are defined here. No logic, no IO, no async. Pure data.

```rust
// models.rs

pub struct CoopMap {
    pub id: i32,
    pub name: String,
    pub version: i32,
    pub filename: String,
}

pub struct Campaign {
    pub id: i32,
    pub name: String,
    pub map_ids: Vec<i32>,   // ordered list of mission IDs
}

pub struct PatchRecord {
    pub file_id: i32,
    pub name: String,
    pub md5: String,
    pub version: i32,
}

pub struct ZipEntry {
    pub path: PathBuf,
    pub content: Vec<u8>,
}

pub struct Asset {
    pub name: String,
    pub download_url: String,
}

pub struct DeployResult {
    pub updated: u32,
    pub skipped: u32,
}
```

---

## 5. Port traits

Each external system has exactly one trait. Services see only the trait — never the
implementation behind it.

```rust
// ports/auth.rs
pub struct CallerIdentity {
    pub user_id: i32,
    pub username: String,
    pub roles: Vec<String>,
}

pub trait AuthPort: Send + Sync {
    async fn verify_token(&self, bearer_token: &str) -> Result<CallerIdentity, AuthError>;
}

// ports/db.rs
pub trait DbPort: Send + Sync {
    async fn get_map_version(&self, map_id: i32) -> Result<Option<i32>, DbError>;
    async fn update_map(&self, map_id: i32, version: i32, filename: &str) -> Result<(), DbError>;
    async fn get_patch_record(&self, file_id: i32) -> Result<Option<PatchRecord>, DbError>;
    async fn upsert_patch(&self, record: PatchRecord) -> Result<(), DbError>;
    async fn list_campaigns(&self) -> Result<Vec<Campaign>, DbError>;
    async fn upsert_campaign(&self, campaign: Campaign) -> Result<(), DbError>;
}

// ports/git.rs
pub trait GitPort: Send + Sync {
    async fn checkout(&self, url: &str, git_ref: &str, workdir: &Path) -> Result<(), GitError>;
}

// ports/fs.rs
pub trait FsPort: Send + Sync {
    async fn write_zip(&self, entries: Vec<ZipEntry>, dest: &Path) -> Result<(), FsError>;
    async fn compute_md5(&self, path: &Path) -> Result<String, FsError>;
    async fn copy_file(&self, src: &Path, dest: &Path) -> Result<(), FsError>;
    async fn list_files(&self, dir: &Path) -> Result<Vec<PathBuf>, FsError>;
}

// ports/github.rs
pub trait GithubPort: Send + Sync {
    async fn fetch_release_assets(&self, repo: &str, tag: &str) -> Result<Vec<Asset>, GithubError>;
    async fn download_asset(&self, url: &str) -> Result<Vec<u8>, GithubError>;
}
```

---

## 6. Services — business logic only

Services receive a `Ports` bundle and return results. No imports from `sqlx`, `reqwest`, or
any IO crate. Testable by swapping in fakes.

```rust
// services/map.rs
pub async fn deploy_all_maps(ports: &Ports, config: &MapConfig) -> Result<DeployResult, DeployError> {
    ports.git.checkout(&config.repo_url, &config.git_ref, &config.workdir).await?;

    let mut updated = 0;
    let mut skipped = 0;

    for map in KNOWN_MAPS {
        let current_version = ports.db.get_map_version(map.id).await?.unwrap_or(0);
        let entries = collect_map_entries(&ports.fs, &config.workdir, map).await?;
        let new_md5 = checksum_entries(&entries);

        if new_md5 == existing_md5(&ports, map.id, current_version).await? {
            skipped += 1;
            continue;
        }

        let next_version = current_version + 1;
        let zip_name = format!("{}.v{:04}.zip", map.name, next_version);
        ports.fs.write_zip(entries, &config.map_dir.join(&zip_name)).await?;
        ports.db.update_map(map.id, next_version, &format!("maps/{zip_name}")).await?;
        updated += 1;
    }

    Ok(DeployResult { updated, skipped })
}
```

```rust
// services/auth.rs
pub async fn require_role(ports: &Ports, token: &str, role: &str) -> Result<CallerIdentity, AuthError> {
    let identity = ports.auth.verify_token(token).await?;
    if !identity.roles.contains(&role.to_string()) {
        return Err(AuthError::forbidden(format!("role '{role}' required")));
    }
    Ok(identity)
}
```

---

## 7. HTTP layer (axum)

Handlers are thin. They do three things: parse the request, call a service, return a response.
No business logic lives here.

```rust
// http/handlers/maps.rs
pub async fn deploy_maps(
    State(ports): State<Arc<Ports>>,
    Extension(caller): Extension<CallerIdentity>,
) -> Result<Json<DeployResult>, AppError> {
    let result = map_service::deploy_all_maps(&ports, &MapConfig::from_env()).await?;
    Ok(Json(result))
}
```

Auth middleware runs before every handler and injects `CallerIdentity` into the request
extensions. Handlers never touch tokens or roles directly.

---

## 8. Auth flow

```
Client sends: Authorization: Bearer <FAF_ACCESS_TOKEN>
    │
    ▼
Middleware calls AuthPort::verify_token(token)
    │
    ▼
HydraAuth fetches JWKS from FAF Hydra (cached, refreshed on key rotation)
    │
    ▼
JWT signature verified, expiry checked, claims extracted
    │
    ▼
CallerIdentity { user_id, username, roles } injected into request
    │
    ▼
Service calls require_role(identity, "COOP_DEPLOYER")
    │
    ├── has role → proceed
    └── no role  → 403 Forbidden
```

The OAuth token is the same token a user gets from the FAF client login (FAF Hydra,
Authorization Code + PKCE). No separate login system needed.

---

## 9. Testing strategy

The fake infrastructure is the key. Every test runs against fakes — no database, no network,
no filesystem.

```rust
// tests/map_deploy.rs
#[tokio::test]
async fn changed_map_increments_version() {
    let ports = fake_ports(); // FakeDb + FakeGit + FakeFs + ...
    ports.db.seed_map(CoopMap { id: 1, version: 3, .. });
    ports.fs.seed_map_files(1, different_content());

    let result = map_service::deploy_all_maps(&ports, &test_config()).await.unwrap();

    assert_eq!(result.updated, 1);
    assert_eq!(ports.db.get_map_version(1).await.unwrap(), Some(4));
}

#[tokio::test]
async fn unchanged_map_is_skipped() {
    let ports = fake_ports();
    ports.db.seed_map(CoopMap { id: 1, version: 3, .. });
    ports.fs.seed_map_files(1, same_content_as_version_3());

    let result = map_service::deploy_all_maps(&ports, &test_config()).await.unwrap();

    assert_eq!(result.skipped, 1);
    assert_eq!(ports.db.get_map_version(1).await.unwrap(), Some(3)); // unchanged
}
```

| Test type | Uses | What it proves |
|-----------|------|----------------|
| Unit (service) | FakePorts | Business logic correctness |
| Integration (handler) | FakePorts + axum TestClient | HTTP layer wiring |
| End-to-end (Phase 6) | Real ports + faf-stack Docker | Full stack against real DB |

---

## 10. Configuration (environment variables)

| Variable | Meaning | Required |
|----------|---------|----------|
| `DATABASE_URL` | PostgreSQL connection string | prod only |
| `FAF_HYDRA_BASE` | Hydra base URL for JWKS | prod only |
| `COOP_MAP_REPO` | GitHub URL for faf-coop-maps | yes |
| `COOP_PATCH_REPO` | GitHub URL for fa-coop | yes |
| `MAP_DIR` | Destination directory for map ZIPs | yes |
| `PATCH_VERSION` | Current patch version string | yes |
| `PORT` | HTTP listen port | default: 8080 |
| `DRY_RUN` | `true` = no writes to DB or filesystem | default: false |
| `FAKE_AUTH` | `true` = skip JWT verification | default: false |
| `FAKE_DB` | `true` = in-memory DB | default: false |

`ports_from_env()` reads these and returns either real or fake ports accordingly.
During development: `FAKE_AUTH=true FAKE_DB=true` — the service runs with zero external dependencies.

---

## 11. "Where do I add X?" (cookbook)

| I want to… | …then |
|---|---|
| Add a new deploy operation | New function in the relevant service. No new crate needed. |
| Add a new external system | Port trait in `ports/`, real impl in `infra/`, fake in `infra/*_fake.rs`, field in `Ports`. |
| Add a new endpoint | Handler in `http/handlers/`, wire into router in `http/mod.rs`. |
| Add a new domain type | `coop-domain/src/models.rs`. No IO, no async. |
| Change DB schema | Update `DbPort` trait + `SqlxDb` impl + `FakeDb` impl together. |

**Never:** do IO inside a service · put logic inside a handler · skip writing a fake · hardcode credentials.

---

## 12. What we ported from Kotlin, and what is new

| Feature | Source | Notes |
|---------|--------|-------|
| Map ZIP creation + versioning | `CoopMapDeployer.kt` | Logic identical, IO behind `FsPort` + `DbPort` |
| Patch file processing | `CoopDeployer.kt` | MD5 comparison + DB upsert logic preserved |
| Voice-over asset download | `CoopDeployer.kt` | Behind `GithubPort` |
| Campaign management | **New** | No script existed. Grouping missions was fully manual. |
| Auth / role checking | **New** | Scripts had no auth whatsoever. |
| Audit trail | **New** | Who deployed what, when. Optional Phase 7+ addition. |
