# FAF Co-op Deployer — Plan

> **Ziel:** Ein HTTP-Web-Service in Rust, der das manuelle Co-op-Deployment von Brutus/Sheikah
> ablöst. Campaign-Team-Mitglieder mit dem Recht `COOP_DEPLOYER` können Maps und Kampagnen
> selbst deployen — ohne Datenbankzugang, ohne Shell-Zugriff.
>
> **Architektur-Prinzip:** Dieselbe Ports/Infra/Services-Trennung wie im rust-client.
> Services tun IO nur über Port-Traits. Infra ist die einzige Zone mit echtem IO.
> Alles testbar ohne Netzwerk.

---

## Ist-Zustand (was wir ersetzen)

Die zwei Kotlin-Skripte im `gitops-stack`:

- **`CoopMapDeployer.kt`** — klont `faf-coop-maps`, berechnet Checksums, erstellt ZIPs,
  schreibt Version + Pfad direkt per SQL in `coop_map`.
- **`CoopDeployer.kt`** — lädt Voice-Over-Assets von GitHub Releases, verarbeitet Patch-Dateien,
  schreibt per SQL in `updates_coop_files`.

Beide Skripte laufen manuell von Brutus/Sheikah. Kein Interface, kein Zugriffsschutz, kein Audit-Log.

---

## Soll-Zustand (was wir bauen)

Ein Rust HTTP-Service (`axum`) mit:

- **OAuth-geschützten Endpunkten** — FAF Hydra Token wird verifiziert, Rolle `COOP_DEPLOYER` geprüft
- **Map-Deploy-Endpunkt** — ersetzt `CoopMapDeployer.kt`
- **Patch-Deploy-Endpunkt** — ersetzt `CoopDeployer.kt`
- **Kampagnen-Management** — Missionen zu Kampagnen gruppieren (bisher komplett manuell, kein Skript)
- **Direkter DB-Zugang** — `sqlx` gegen dieselben Tabellen wie die Kotlin-Skripte
- **Fake-Implementierungen** — für alle externen Systeme, sodass lokal ohne echte DB entwickelt werden kann

---

## Crate-Struktur

```
faf-coop-deployer/
├── Cargo.toml                  # workspace
├── crates/
│   ├── coop-domain/            # PURE. Typen, keine IO, kein async.
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs       # CoopMap, Campaign, PatchFile, DeployResult, ...
│   │   │   └── errors.rs       # Domänen-Fehlertypen
│   │   └── Cargo.toml
│   │
│   └── coop-app/               # Alle async + IO.
│       ├── src/
│       │   ├── lib.rs
│       │   ├── ports/
│       │   │   ├── mod.rs      # Ports-Bundle
│       │   │   ├── auth.rs     # AuthPort: verify_token(token) -> CallerIdentity
│       │   │   ├── db.rs       # DbPort: get_map_version, update_map, update_patch, ...
│       │   │   ├── git.rs      # GitPort: clone_repo(url, ref) -> LocalPath
│       │   │   ├── fs.rs       # FsPort: write_zip, compute_checksum, copy_file, ...
│       │   │   └── github.rs   # GithubPort: fetch_release_assets(repo, tag) -> Vec<Asset>
│       │   ├── infra/
│       │   │   ├── mod.rs      # real_ports() / fake_ports() / ports_from_env()
│       │   │   ├── auth.rs     # HydraAuth — JWT verifizieren gegen FAF Hydra JWKS
│       │   │   ├── auth_fake.rs# FakeAuth — akzeptiert jeden Token, gibt Fake-Identity zurück
│       │   │   ├── db.rs       # SqlxDb — sqlx + PostgreSQL, echte Queries
│       │   │   ├── db_fake.rs  # FakeDb — in-memory HashMap, kein DB nötig
│       │   │   ├── git.rs      # GitInfra — git2 crate, klont Repos
│       │   │   ├── git_fake.rs # FakeGit — gibt fixtures aus dem Dateisystem zurück
│       │   │   ├── fs.rs       # LocalFs — echte Dateisystem-Operationen
│       │   │   ├── fs_fake.rs  # FakeFs — in-memory
│       │   │   └── github.rs   # GithubClient — reqwest gegen GitHub API
│       │   ├── services/
│       │   │   ├── mod.rs
│       │   │   ├── auth.rs     # Token verifizieren + Rolle prüfen
│       │   │   ├── map.rs      # Map-Deploy-Logik (portiert von CoopMapDeployer.kt)
│       │   │   ├── patch.rs    # Patch-Deploy-Logik (portiert von CoopDeployer.kt)
│       │   │   └── campaign.rs # Kampagnen-Gruppierung (neu — bisher kein Skript)
│       │   └── http/
│       │       ├── mod.rs      # axum Router zusammenbauen
│       │       ├── middleware.rs# Auth-Middleware: Token aus Header → CallerIdentity
│       │       └── handlers/
│       │           ├── maps.rs     # POST /maps/deploy
│       │           ├── patches.rs  # POST /patches/deploy
│       │           └── campaigns.rs# GET/POST/PUT /campaigns
│       └── Cargo.toml
│
├── src/
│   └── main.rs                 # Einstiegspunkt: ports_from_env() + axum server starten
├── tests/
│   ├── map_deploy.rs           # Integration: FakePorts, echter Service, kein Netzwerk
│   ├── patch_deploy.rs
│   └── campaign.rs
└── docs/
    ├── PLAN.md                 # Dieses Dokument
    └── ARCHITECTURE.md         # (wird nach Phase 1 geschrieben)
```

---

## Port-Traits (die Kernabstraktion)

```rust
// ports/auth.rs
pub struct CallerIdentity {
    pub user_id: i32,
    pub username: String,
    pub roles: Vec<String>,
}

pub trait AuthPort: Send + Sync {
    async fn verify_token(&self, bearer_token: &str) -> Result<CallerIdentity, AuthError>;
    fn has_role(identity: &CallerIdentity, role: &str) -> bool {
        identity.roles.contains(&role.to_string())
    }
}

// ports/db.rs
pub trait DbPort: Send + Sync {
    async fn get_map_version(&self, map_id: i32) -> Result<Option<i32>, DbError>;
    async fn update_map(&self, map_id: i32, version: i32, filename: &str) -> Result<(), DbError>;
    async fn get_patch_version(&self, file_id: i32) -> Result<Option<PatchRecord>, DbError>;
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
    async fn copy_file(&self, src: &Path, dest: &Path, mode: u32) -> Result<(), FsError>;
}

// ports/github.rs
pub trait GithubPort: Send + Sync {
    async fn fetch_release_assets(&self, repo: &str, tag: &str) -> Result<Vec<Asset>, GithubError>;
}
```

---

## HTTP-Endpunkte

| Method | Path | Was es tut |
|--------|------|-----------|
| `POST` | `/maps/deploy` | Klont `faf-coop-maps`, verarbeitet alle 31 Maps, updated DB |
| `POST` | `/maps/deploy/:map_id` | Einzelne Map deployen |
| `GET` | `/campaigns` | Alle Kampagnen auflisten |
| `POST` | `/campaigns` | Neue Kampagne erstellen |
| `PUT` | `/campaigns/:id` | Kampagne aktualisieren (Missionsreihenfolge etc.) |
| `POST` | `/patches/deploy` | Voice-Overs + Patch-Dateien deployen |

Alle Endpunkte: Bearer Token required, Rolle `COOP_DEPLOYER` required.

---

## Phase-Plan

### Phase 0 — Workspace-Skeleton (kein echter Code)

**Ziel:** Alles kompiliert, Struktur steht, CI läuft.

- [ ] `Cargo.toml` (workspace mit `coop-domain` + `coop-app`)
- [ ] `coop-domain/src/lib.rs` — leer, kompiliert
- [ ] `coop-app/src/lib.rs` — leer, kompiliert
- [ ] `src/main.rs` — `fn main() { println!("ok"); }`
- [ ] `.github/workflows/ci.yml` — `cargo test` + `cargo clippy -D warnings`
- [ ] `.gitignore`
- [ ] Erstes Commit, Repo auf GitHub

**Ergebnis:** Grüne CI, leeres aber korrektes Workspace.

---

### Phase 1 — Domain-Typen (pure, kein IO)

**Ziel:** Alle Datentypen definiert, keine Logik, keine IO.

- [ ] `coop-domain/src/models.rs`:
  - `CoopMap { id: i32, name: String, version: i32, filename: String }`
  - `Campaign { id: i32, name: String, map_ids: Vec<i32> }`
  - `PatchRecord { file_id: i32, name: String, md5: String, version: i32 }`
  - `ZipEntry { path: PathBuf, content: Vec<u8> }`
  - `Asset { name: String, download_url: String }`
  - `DeployResult { maps_updated: u32, maps_skipped: u32 }`
- [ ] `coop-domain/src/errors.rs`:
  - `AuthError`, `DbError`, `GitError`, `FsError`, `GithubError`, `DeployError`
- [ ] Unit-Tests für alle Typen (Serialisierung, Grundoperationen)

**Ergebnis:** `coop-domain` ist ein stabiles, dependency-freies Fundament.

---

### Phase 2 — Port-Traits + Fake-Implementierungen

**Ziel:** Alle Port-Traits definiert, Fakes implementiert. Kein echter DB/Netzwerk-Code.

- [ ] `coop-app/src/ports/auth.rs` — `AuthPort` trait + `CallerIdentity`
- [ ] `coop-app/src/ports/db.rs` — `DbPort` trait
- [ ] `coop-app/src/ports/git.rs` — `GitPort` trait
- [ ] `coop-app/src/ports/fs.rs` — `FsPort` trait
- [ ] `coop-app/src/ports/github.rs` — `GithubPort` trait
- [ ] `coop-app/src/ports/mod.rs` — `Ports` Bundle
- [ ] `coop-app/src/infra/auth_fake.rs` — akzeptiert jeden Token, gibt konfigurierbaren User zurück
- [ ] `coop-app/src/infra/db_fake.rs` — HashMap im Speicher, kein SQL
- [ ] `coop-app/src/infra/git_fake.rs` — kopiert Test-Fixtures statt zu klonen
- [ ] `coop-app/src/infra/fs_fake.rs` — in-memory statt Dateisystem
- [ ] `coop-app/src/infra/github_fake.rs` — gibt statische Test-Assets zurück
- [ ] `coop-app/src/infra/mod.rs` — `fake_ports()` zusammenbauen

**Ergebnis:** Alle Services können jetzt ohne Netzwerk/DB getestet werden.

---

### Phase 3 — Services (Kernlogik)

**Ziel:** Die eigentliche Deploy-Logik, portiert von Kotlin nach Rust. Nur Fakes, kein echtes IO.

**3a — Map-Deploy-Service** (portiert von `CoopMapDeployer.kt`):
- [ ] `services/map.rs::deploy_all_maps(ports, workdir)`:
  - für jede der 31 Maps: Checksums berechnen, mit DB vergleichen
  - falls geändert: Version hochzählen, ZIP erstellen, DB updaten
  - falls unverändert: überspringen
- [ ] Test: `FakePorts` → `deploy_all_maps` → `FakeDb` prüfen ob Version hochgezählt

**3b — Patch-Deploy-Service** (portiert von `CoopDeployer.kt`):
- [ ] `services/patch.rs::deploy_patches(ports, version, repo_url, git_ref)`:
  - Voice-Over-Assets von GitHub Release laden
  - Patch-Dateien verarbeiten, MD5 vergleichen
  - falls geändert: DB updaten
- [ ] Test: `FakePorts` → `deploy_patches` → Ergebnis prüfen

**3c — Kampagnen-Service** (neu):
- [ ] `services/campaign.rs`:
  - `list_campaigns(ports)` → alle Kampagnen aus DB
  - `create_campaign(ports, name, map_ids)` → neue Kampagne in DB
  - `update_campaign(ports, id, map_ids)` → Missionsreihenfolge ändern
- [ ] Tests für alle drei Operationen

**3d — Auth-Service**:
- [ ] `services/auth.rs::require_role(ports, token, role)`:
  - Token verifizieren via `AuthPort`
  - Rolle prüfen, sonst `403 Forbidden`
- [ ] Test: gültiger Token mit Rolle → ok; ohne Rolle → Fehler; ungültiger Token → Fehler

**Ergebnis:** Gesamte Business-Logik fertig und getestet, ohne eine Zeile echter DB/HTTP-Code.

---

### Phase 4 — HTTP-Layer (axum)

**Ziel:** REST-API, die die Services exposed.

- [ ] `http/middleware.rs` — extrahiert Bearer Token aus `Authorization` Header, ruft `auth_service::require_role`, hängt `CallerIdentity` an den Request-Context
- [ ] `http/handlers/maps.rs`:
  - `POST /maps/deploy` → ruft `map_service::deploy_all_maps`
  - `POST /maps/deploy/:map_id` → einzelne Map
- [ ] `http/handlers/patches.rs`:
  - `POST /patches/deploy` → ruft `patch_service::deploy_patches`
- [ ] `http/handlers/campaigns.rs`:
  - `GET /campaigns` → `campaign_service::list_campaigns`
  - `POST /campaigns` → `campaign_service::create_campaign`
  - `PUT /campaigns/:id` → `campaign_service::update_campaign`
- [ ] `http/mod.rs` — axum Router zusammenbauen, Ports injizieren
- [ ] Integration-Tests mit `FakePorts` + axum `TestClient`

**Ergebnis:** Vollständige API, lokal testbar ohne DB oder Netzwerk.

---

### Phase 5 — Echte Infra-Implementierungen

**Ziel:** Echtes IO hinter den Port-Traits. Fakes bleiben für Tests.

- [ ] `infra/auth.rs` — `HydraAuth`:
  - JWKS von FAF Hydra laden und cachen
  - JWT verifizieren (Signatur + Ablaufzeit)
  - Rollen aus Token-Claims extrahieren
- [ ] `infra/db.rs` — `SqlxDb`:
  - `sqlx` + PostgreSQL
  - Connection Pool
  - Echte SQL-Queries (portiert von den Kotlin-Skripten)
  - Tabellen: `coop_map`, `updates_coop_files`, (neue Tabelle für Kampagnen)
- [ ] `infra/git.rs` — `GitInfra`:
  - `git2` crate
  - Klonen + Checkout
- [ ] `infra/fs.rs` — `LocalFs`:
  - ZIP erstellen (`zip` crate)
  - MD5 berechnen
  - Dateien kopieren mit Permissions
- [ ] `infra/github.rs` — `GithubClient`:
  - `reqwest` gegen GitHub API
  - Release-Assets abrufen und herunterladen
- [ ] `infra/mod.rs` — `real_ports()` zusammenbauen
- [ ] `src/main.rs` — `ports_from_env()` + axum server auf Port aus Env-Variable

**Ergebnis:** Service läuft vollständig lokal gegen `faf-stack` Docker-Umgebung.

---

### Phase 6 — Lokaler End-to-End-Test

**Ziel:** Gegen die echte lokale DB testen (faf-stack Docker).

- [ ] `faf-stack` lokal aufsetzen (oder neues `gitops-stack` Tilt-Setup)
- [ ] `DATABASE_URL` + `FAF_HYDRA_BASE` Env-Variablen setzen
- [ ] `/maps/deploy` gegen lokale DB aufrufen
- [ ] Prüfen ob `coop_map` Tabelle korrekt aktualisiert wird
- [ ] Neue Kampagne anlegen, abrufen, aktualisieren

---

### Phase 7 — Übergabe an FAF

**Ziel:** Service produktionsbereit, FAF-Team kann es integrieren.

- [ ] `COOP_DEPLOYER` Rolle in `UserRole.java` (faf-java-api) hinzufügen — Brutus/Sheikah
- [ ] Deployment-Config für `gitops-stack` (Kubernetes Manifest / Helm Chart)
- [ ] Secrets-Management (DB-Credentials, kein Hardcoding)
- [ ] Mordor: neue Rolle über UI vergeben können
- [ ] PR an FAF-Repos, Review durch Brutus/Sheikah/Jip

---

## Abhängigkeiten (Cargo)

```toml
# coop-app
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio"] }
reqwest = { version = "0.11", features = ["json"] }
git2 = "0.18"
zip = "0.6"
jsonwebtoken = "9"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
md5 = "0.7"
anyhow = "1"
thiserror = "1"
async-trait = "0.1"
tower = "0.4"
tower-http = { version = "0.5", features = ["auth"] }
```

---

## Env-Variablen (Konfiguration)

| Variable | Bedeutung | Default |
|----------|-----------|---------|
| `DATABASE_URL` | PostgreSQL Connection String | — (required in prod) |
| `FAF_HYDRA_BASE` | Hydra JWKS Endpoint Base URL | `https://hydra.faforever.com` |
| `COOP_MAP_REPO` | GitHub URL für faf-coop-maps | `https://github.com/FAForever/faf-coop-maps` |
| `COOP_PATCH_REPO` | GitHub URL für fa-coop | aus Kotlin-Skript |
| `PATCH_VERSION` | Aktuelle Patch-Version | — |
| `MAP_DIR` | Ziel-Verzeichnis für Map-ZIPs | — |
| `DRY_RUN` | `true` = keine echten Schreiboperationen | `false` |
| `FAKE_AUTH` | `true` = FakeAuth statt Hydra | `false` |
| `FAKE_DB` | `true` = FakeDb statt PostgreSQL | `false` |
| `PORT` | HTTP Port | `8080` |

---

## Was wir NICHT von Brutus/Sheikah brauchen um anzufangen

- Phase 0–4 sind komplett unabhängig von FAF-Infrastruktur
- Fakes erlauben Entwicklung ohne DB-Zugang
- Erst ab Phase 5 brauchen wir Antworten zur DB-Struktur

## Was wir von Brutus/Sheikah brauchen (für Phase 5+)

1. Exaktes DB-Schema von `coop_map` und `updates_coop_files`
2. Lokales Docker-Setup (faf-stack) bestätigen dass Co-op-Tabellen drin sind
3. `COOP_DEPLOYER` in `UserRole.java` hinzufügen (wenn wir Phase 7 erreichen)
