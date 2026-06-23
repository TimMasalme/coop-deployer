# FAF Co-op Deployer — Plan

> **Goal:** A Rust HTTP service that replaces the manual co-op deployment scripts run by Brutus/Sheikah.
> Campaign team members with the `COOP_DEPLOYER` role can deploy maps and campaigns themselves —
> no database access required, no shell access required.
>
> **Architecture principle:** The same Ports/Infra/Services separation as in the rust-client.
> Services do IO only through Port traits. Infra is the only zone with real IO.
> Everything is testable without network access.

---

## Current state (what we replace)

Two Kotlin scripts in `gitops-stack`:

- **`CoopMapDeployer.kt`** — clones `faf-coop-maps`, computes checksums, creates ZIPs,
  writes version + path directly via SQL into `coop_map`.
- **`CoopDeployer.kt`** — downloads voice-over assets from GitHub Releases, processes patch files,
  writes via SQL into `updates_coop_files`.

Both scripts are run manually by Brutus/Sheikah. No interface, no access control, no audit log.

---

## Target state (what we build)

A Rust HTTP service (`axum`) with:

- **OAuth-protected endpoints** — FAF Hydra token is verified, `COOP_DEPLOYER` role checked
- **Map deploy endpoint** — replaces `CoopMapDeployer.kt`
- **Patch deploy endpoint** — replaces `CoopDeployer.kt`
- **Campaign management** — group missions into campaigns (previously fully manual, no script)
- **Direct DB access** — `sqlx` against the same tables as the Kotlin scripts
- **Fake implementations** — for all external systems, so local development works without a real DB

---

## Crate structure

```
faf-coop-deployer/
├── Cargo.toml                  # workspace
├── crates/
│   ├── coop-domain/            # PURE. Types, no IO, no async.
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs       # CoopMap, Campaign, PatchRecord, ZipEntry, Asset, DeployResult, CallerIdentity
│   │   │   └── errors.rs       # Domain error types
│   │   └── Cargo.toml
│   │
│   └── coop-app/               # All async + IO.
│       ├── src/
│       │   ├── lib.rs
│       │   ├── ports/
│       │   │   ├── mod.rs      # Ports bundle
│       │   │   ├── auth.rs     # AuthPort: verify_token(token) -> CallerIdentity
│       │   │   ├── db.rs       # DbPort: get_map, update_map, get_patch_record, upsert_patch, ...
│       │   │   ├── git.rs      # GitPort: checkout(url, ref, workdir)
│       │   │   ├── fs.rs       # FsPort: write_zip, compute_md5, copy_file
│       │   │   └── github.rs   # GithubPort: fetch_release_assets, download_asset
│       │   ├── infra/
│       │   │   ├── mod.rs      # fake_ports() / ports_from_env()
│       │   │   ├── auth.rs     # HydraAuth — token introspection against FAF Hydra
│       │   │   ├── auth_fake.rs# FakeAuth — accepts any token, returns configurable identity
│       │   │   ├── db.rs       # SqlxDb — sqlx + PostgreSQL, runtime queries (no macros)
│       │   │   ├── db_fake.rs  # FakeDb — in-memory HashMap, no DB needed
│       │   │   ├── git.rs      # GitInfra — git2 crate, clones repos
│       │   │   ├── git_fake.rs # FakeGit — returns fixtures from filesystem
│       │   │   ├── fs.rs       # LocalFs — real filesystem operations
│       │   │   ├── fs_fake.rs  # FakeFs — in-memory
│       │   │   ├── github.rs   # GithubClient — reqwest against GitHub API
│       │   │   ├── github_fake.rs # FakeGithub — returns static test assets
│       │   │   └── test_support.rs # TestPorts: holds Arc<FakeX> for seed methods + .ports()
│       │   ├── services/
│       │   │   ├── mod.rs
│       │   │   ├── auth.rs     # require_role() — token verify + role check
│       │   │   ├── map.rs      # Map deploy logic (ported from CoopMapDeployer.kt)
│       │   │   ├── patch.rs    # Patch deploy logic (ported from CoopDeployer.kt)
│       │   │   └── campaign.rs # Campaign grouping (new — no prior script)
│       │   └── http/
│       │       ├── mod.rs      # axum router
│       │       ├── middleware.rs# Auth middleware: token from header -> CallerIdentity
│       │       └── handlers/
│       │           ├── maps.rs     # POST /maps/deploy
│       │           ├── patches.rs  # POST /patches/deploy
│       │           └── campaigns.rs# GET/POST/PUT /campaigns
│       └── Cargo.toml
│
├── src/
│   └── main.rs                 # Entry point: ports_from_env() + axum server
├── bruno/                      # Bruno API collection for manual testing
│   └── ...
└── docs/
    ├── PLAN.md                 # This document
    └── ARCHITECTURE.md         # Architecture contract
```

---

## Port traits (the core abstraction)

```rust
pub trait AuthPort: Send + Sync {
    async fn verify_token(&self, bearer_token: &str) -> AuthResult<CallerIdentity>;
}

pub trait DbPort: Send + Sync {
    async fn get_map(&self, map_id: i32) -> DbResult<Option<CoopMap>>;
    async fn update_map(&self, map_id: i32, version: i32, filename: &str, checksum: &str) -> DbResult<()>;
    async fn get_patch_record(&self, file_id: i32) -> DbResult<Option<PatchRecord>>;
    async fn upsert_patch(&self, record: PatchRecord) -> DbResult<()>;
    async fn list_campaigns(&self) -> DbResult<Vec<Campaign>>;
    async fn upsert_campaign(&self, campaign: Campaign) -> DbResult<()>;
}

pub trait GitPort: Send + Sync {
    async fn checkout(&self, url: &str, git_ref: &str, workdir: &Path) -> GitResult<()>;
}

pub trait FsPort: Send + Sync {
    async fn write_zip(&self, entries: Vec<ZipEntry>, dest: &Path) -> FsResult<()>;
    async fn compute_md5(&self, path: &Path) -> FsResult<String>;
    async fn copy_file(&self, src: &Path, dest: &Path) -> FsResult<()>;
}

pub trait GithubPort: Send + Sync {
    async fn fetch_release_assets(&self, repo: &str, tag: &str) -> GithubResult<Vec<Asset>>;
    async fn download_asset(&self, url: &str) -> GithubResult<Vec<u8>>;
}
```

---

## HTTP endpoints

| Method | Path | What it does |
|--------|------|-------------|
| `POST` | `/maps/deploy` | Clones `faf-coop-maps`, processes all 31 maps, updates DB |
| `GET` | `/campaigns` | List all campaigns |
| `POST` | `/campaigns` | Create new campaign |
| `PUT` | `/campaigns/:id` | Update campaign (mission order etc.) |
| `POST` | `/patches/deploy` | Deploy voice-overs + patch files |

All endpoints: Bearer token required, `COOP_DEPLOYER` role required.

---

## Phase plan

### Phase 0 — Workspace skeleton [COMPLETE]

**Goal:** Everything compiles, structure in place, CI running.

- [x] `Cargo.toml` (workspace with `coop-domain` + `coop-app` + binary)
- [x] `coop-domain/src/lib.rs` — compiles
- [x] `coop-app/src/lib.rs` — compiles
- [x] `src/main.rs` — entry point with tokio + axum
- [x] `.github/workflows/ci.yml` — `cargo test` + `cargo clippy -D warnings`
- [x] `.gitignore`

---

### Phase 1 — Domain types [COMPLETE]

**Goal:** All data types defined, no logic, no IO.

- [x] `coop-domain/src/models.rs`:
  - `CoopMap { id, name, version, filename, checksum }` (checksum added: parsed from filename)
  - `Campaign { id, name, map_ids }`
  - `PatchRecord { file_id, name, md5, version }`
  - `ZipEntry { path, content }`
  - `Asset { name, download_url }`
  - `DeployResult { updated, skipped }`
  - `CallerIdentity { user_id, username, roles }` with `has_role()` method
- [x] `coop-domain/src/errors.rs`:
  - `AuthError`, `DbError`, `GitError`, `FsError`, `GithubError`, `DeployError`

**Note:** `CoopMap` gained a `checksum` field compared to the original plan. Checksum is stored
in the filename (`name.v0001.abc12345.zip`, 8-char MD5 prefix) since `coop_map` has no checksum
column. `SqlxDb` parses it back out; `FakeDb` stores the full MD5 directly.

---

### Phase 2 — Port traits + fake implementations [COMPLETE]

**Goal:** All port traits defined, fakes implemented. No real DB/network code.

- [x] `ports/auth.rs` — `AuthPort` trait
- [x] `ports/db.rs` — `DbPort` trait
- [x] `ports/git.rs` — `GitPort` trait
- [x] `ports/fs.rs` — `FsPort` trait
- [x] `ports/github.rs` — `GithubPort` trait (includes `download_asset` — added vs. original plan)
- [x] `ports/mod.rs` — `Ports` bundle
- [x] `infra/auth_fake.rs` — accepts any token, returns configurable user with roles
- [x] `infra/db_fake.rs` — HashMap in memory, no SQL
- [x] `infra/git_fake.rs` — copies test fixtures instead of cloning
- [x] `infra/fs_fake.rs` — in-memory instead of filesystem
- [x] `infra/github_fake.rs` — returns static test assets
- [x] `infra/test_support.rs` — `TestPorts` struct holding `Arc<FakeX>` directly for seed methods;
  `.ports()` converts to `Ports` bundle. Used in all service tests.
- [x] `infra/mod.rs` — `fake_ports()` assembled

---

### Phase 3 — Services (core logic) [COMPLETE]

**Goal:** The actual deploy logic, ported from Kotlin to Rust. Fakes only, no real IO.

**3a — Map deploy service** (ported from `CoopMapDeployer.kt`):
- [x] `services/map.rs::deploy_all_maps(ports, workdir)`:
  - `KNOWN_MAPS` const with all 31 co-op maps and their DB IDs
  - for each map: compute checksum, compare with DB (8-char MD5 prefix)
  - if changed: increment version, create ZIP with hash in filename, update DB
  - if unchanged: skip
- [x] Tests: unchanged map is skipped; changed map increments version

**3b — Patch deploy service** (ported from `CoopDeployer.kt`):
- [x] `services/patch.rs::deploy_patches(ports)`:
  - `PATCH_FILES` const with all 25 patch files
  - downloads voice-over assets from GitHub release
  - compares MD5, updates DB if changed
  - `PATCH_VERSION` env var: defaults to `"latest"` / version 1 in dev (no hard error)
- [x] Tests: patch deploy with fake ports

**3c — Campaign service** (new):
- [x] `services/campaign.rs`:
  - `list_campaigns(ports)` — all campaigns from DB
  - `create_campaign(ports, name, map_ids)` — new campaign in DB
  - `update_campaign(ports, id, map_ids)` — update mission order
- [x] Tests for all three operations

**3d — Auth service**:
- [x] `services/auth.rs::require_role(ports, token, role)`:
  - verify token via `AuthPort`
  - check role, otherwise `403 Forbidden`
  - `ROLE_COOP_DEPLOYER = "COOP_DEPLOYER"` constant
- [x] Tests: valid token with role -> ok; without role -> error; invalid token -> error

---

### Phase 4 — HTTP layer (axum) [COMPLETE]

**Goal:** REST API exposing the services.

- [x] `http/middleware.rs` — extracts Bearer token from `Authorization` header, calls
  `auth_service::require_role`, injects `CallerIdentity` into request extensions
- [x] `http/handlers/maps.rs` — `POST /maps/deploy` -> `map_service::deploy_all_maps`
- [x] `http/handlers/patches.rs` — `POST /patches/deploy` -> `patch_service::deploy_patches`
- [x] `http/handlers/campaigns.rs`:
  - `GET /campaigns` -> `campaign_service::list_campaigns`
  - `POST /campaigns` -> `campaign_service::create_campaign`
  - `PUT /campaigns/:id` -> `campaign_service::update_campaign`
- [x] `http/mod.rs` — axum router assembled, Ports injected via `Arc`
- [x] Bruno collection with all 5 endpoints manually verified (`FAKE_AUTH=true`, `FAKE_DB=true`)

---

### Phase 5 — Real infra implementations [COMPLETE]

**Goal:** Real IO behind the port traits. Fakes remain for tests.

- [x] `infra/auth.rs` — `HydraAuth`:
  - Uses `/oauth2/introspect` endpoint (simpler than JWKS)
  - Parses `sub` (user ID), `ext.username`, `ext.roles` from response
  - `FAF_HYDRA_BASE` env var, defaults to `https://hydra.faforever.com`
- [x] `infra/db.rs` — `SqlxDb`:
  - `sqlx` + PostgreSQL with connection pool
  - Runtime `sqlx::query()` calls — no compile-time macros (avoids `DATABASE_URL` at build time)
  - SQL matches Kotlin scripts exactly, including the `obselete` typo in `updates_coop_files`
  - Campaign methods stubbed: `list_campaigns` returns empty vec, `upsert_campaign` returns error
    (DB migration required — coordinate with Brutus/Sheikah)
- [x] `infra/git.rs` — `GitInfra`: `git2` crate, clone or fetch existing repo
- [x] `infra/fs.rs` — `LocalFs`: ZIP creation (`zip` crate, Deflated), MD5 (`md5` crate)
- [x] `infra/github.rs` — `GithubClient`: `reqwest`, `User-Agent: faf-coop-deployer/1.0`,
  fetches release assets + downloads binary content
- [x] `infra/mod.rs` — `ports_from_env()`:
  - `FAKE_AUTH=true` -> `FakeAuth`, else `HydraAuth::faf()`
  - `FAKE_DB=true` -> `FakeDb`, else `SqlxDb::connect(DATABASE_URL)`
- [x] `src/main.rs` — `ports_from_env()` + axum server on `$PORT` (default 8080)
- [x] `Dockerfile` — two-stage build: `rust:1.82-slim` builder -> `debian:bookworm-slim` runtime

---

### Phase 6 — Local end-to-end test [BLOCKED]

**Goal:** Test against the real local DB (faf-stack Docker).

- [ ] Set up `faf-stack` locally (or new `gitops-stack` Tilt setup)
- [ ] Set `DATABASE_URL` + `FAF_HYDRA_BASE` env vars
- [ ] Call `/maps/deploy` against local DB
- [ ] Verify `coop_map` table is correctly updated
- [ ] Create, fetch, and update a campaign

**Blocked on:** Brutus/Sheikah confirming local faf-stack setup with co-op tables.

---

### Phase 7 — FAF handoff [NOT STARTED]

**Goal:** Service production-ready, FAF team can integrate it.

- [ ] Add `COOP_DEPLOYER` role to `UserRole.java` (faf-java-api) — needs Brutus/Sheikah
- [ ] Deployment config for `gitops-stack` (Kubernetes manifest / Helm chart)
- [ ] Secrets management (DB credentials, no hardcoding)
- [ ] Mordor: new role assignable via UI
- [ ] DB migration for campaign table (new `coop_campaign` table)
- [ ] PR to FAF repos, review by Brutus/Sheikah/Jip

---

## Environment variables

| Variable | Meaning | Default |
|----------|---------|---------|
| `DATABASE_URL` | PostgreSQL connection string | — (required in prod) |
| `FAF_HYDRA_BASE` | Hydra introspect endpoint base URL | `https://hydra.faforever.com` |
| `COOP_MAP_REPO` | GitHub URL for faf-coop-maps | `https://github.com/FAForever/faf-coop-maps` |
| `PATCH_VERSION` | Current patch version tag | defaults to `latest` / version 1 in dev |
| `MAP_DIR` | Target directory for map ZIPs | — |
| `FAKE_AUTH` | `true` = FakeAuth instead of Hydra | `false` |
| `FAKE_DB` | `true` = FakeDb instead of PostgreSQL | `false` |
| `PORT` | HTTP port | `8080` |

---

## What we do NOT need from Brutus/Sheikah to proceed

- Phases 0–5 are completely independent of FAF infrastructure
- Fakes allow development without DB access
- CI passes with `cargo test` + `cargo clippy -D warnings`

## What we need from Brutus/Sheikah (for Phase 6+)

1. Confirm local faf-stack Docker setup includes `coop_map` and `updates_coop_files` tables
2. Add `COOP_DEPLOYER` to `UserRole.java` in faf-java-api (Phase 7)
3. Review and merge PRs for gitops-stack deployment config (Phase 7)
4. Coordinate DB migration for `coop_campaign` table (Phase 7)
