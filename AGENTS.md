# AGENTS.md — arcana-cloud-rust

Instructions for any AI coding agent (Claude Code, Codex, Cursor, Copilot, Gemini…) working
in this repository **or in any product created from it**. `CLAUDE.md`, `GEMINI.md` and
`.github/copilot-instructions.md` all point here; this file is the single source.

> **You MUST NOT report a task as done until `tools/arch-check.sh` exits 0 and the CI test
> lane (§4) is green.** Breaking the layering costs a red build: the pre-commit hook
> (`tools/githooks/pre-commit`) rejects new flat files and `*Impl` references outside the DI
> container, and Jenkins stage "Architecture Qube" fails the merge below score 90.

## 0. First command in a fresh clone

Run `tools/install-hooks.sh` once. Git does not install hooks on clone; until you run it the
pre-commit check does not exist and only CI catches a flat layout — after the work is
written. `tools/new-app.sh` runs it for you.

## 1. Starting something new

| Situation | Do this | Never do this |
|---|---|---|
| New service product | `tools/new-app.sh <name> <dest-dir>` — copies the whole workspace, renames crates `arcana-*` → `<name>-*`, CI and Sonar identities, installs the hook | `cargo new` and copy a few crates or files into it |
| New business feature (entity + CRUD API) | `tools/new-feature.sh <Name>` — one vertical slice: entity, DAO trait + MySQL impl, repository trait + impl, DTOs + mapper, service trait + impl, controller, migration, tests, DI + router + OpenAPI wiring | Write handlers that run SQL, or a new `foo.rs` next to `lib.rs` |
| Feature over gRPC / layered deployment | After `new-feature.sh`, mirror the User feature by hand: `proto/user_service.proto`, `crates/*-grpc/src/services/user_service.rs`, `crates/*-repository/src/remote/` | Call the DAO from the gRPC service |

Copying parts instead of `new-app.sh` loses this file, the hook and the CI gate, so nothing
tells the next agent the rules exist.

## 2. Layers — where a file goes

Paths use `crates/*-<layer>` because `new-app.sh` renames the `arcana-` prefix.

| Layer | Path | Contains | May depend on |
|---|---|---|---|
| Entry + DI | `crates/*-server/src/main.rs`, `crates/*-server/src/di.rs`, `crates/*-grpc/src/di.rs` | startup, Shaku `module!` definitions | everything; the only place that names `*Impl` types |
| REST boundary | `crates/*-rest/src/controllers/<x>_controller.rs`, `router.rs`, `openapi.rs`, `extractors/`, `middleware/`, `responses/` | Axum handlers: extract → call service trait → wrap DTO | service traits + DTOs, core, security |
| gRPC boundary | `crates/*-grpc/src/services/`, `clients/`, `interceptors/`; messages in `proto/` → `crates/*-proto` | Tonic services and clients | service traits + DTOs, proto, core |
| Service | trait `crates/*-service/src/<x>_service.rs`; impl `src/impl/<x>_service_impl.rs`; `src/dto/`, `src/mappers/`, `src/cache/` | business rules, validation, DTO mapping | repository traits, core, security, config |
| Repository | traits in `crates/*-repository/src/traits.rs`; impls in `src/impl/<x>_repository_impl.rs`; gRPC-backed in `src/remote/` | domain-level data access, coordinates DAOs | DAO traits, core |
| DAO | trait `crates/*-repository/src/dao/<x>_dao.rs`; impl `src/dao/impl/mysql/<x>_dao_impl.rs`; schema in `migrations/` | one data source per impl, SQLx queries | core, `DatabasePoolInterface` |
| Core | `crates/*-core/src/domain/{entities,value_objects,events}/`, `error.rs`, `pagination.rs` | entities, value objects, `ArcanaError`, `Page` | no workspace crate |
| Cross-cutting | `crates/*-{security,config,resilience,jobs,plugin-api,plugin-runtime,ssr-engine}` | JWT/RBAC, config, circuit breaker, job queue, WASM plugins | core, config |

Direction is one way: controller → service → repository → DAO → core. A lower layer never
uses a higher one. Worked example of every layer: the User feature —
`user_controller.rs` → `UserService` → `UserRepository`/`UserRepositoryImpl` →
`UserDao`/`MySqlUserDaoImpl`, wired in `crates/*-server/src/di.rs` `MonolithicModule`.

## 3. Rules

| # | Rule | Enforced by |
|---|---|---|
| R1 | Every service, repository and DAO is a trait (`XService`, `XRepository`, `XDao`, bound `Interface + Send + Sync`) implemented by `XServiceImpl` / `XRepositoryImpl` / `MySqlXDaoImpl` in an `impl/` directory, as a Shaku `#[derive(Component)]` with `#[shaku(inject)] Arc<dyn Trait>` fields. | arch-qube "Interface/Impl Colocation", "Implementation Naming Convention" |
| R2 | Only `di.rs` names a concrete `*Impl` type. Everyone else holds `Arc<dyn Trait>`. Re-exports (`pub use`) in `lib.rs` / `mod.rs` are the only other allowed mention. | arch-qube "Only DI Container Imports Impl"; pre-commit hook |
| R3 | Controllers call services only; services call repositories only; only DAOs run SQL. | arch-qube "Backend Layer Chain", "Controller Cannot Use DAO", "Service Cannot Access DB Directly", "Layer Direction Enforcement" |
| R4 | No new `.rs` file at a crate's `src/` root except `lib.rs`, `main.rs` and service traits `crates/*-service/src/<x>_service.rs`. | pre-commit hook |
| R5 | A handler only extracts, delegates and wraps: ≤ 20 lines, no business logic. Logic goes in the service. | arch-qube "No Business Logic in Boundary" |
| R6 | APIs take and return DTOs from `crates/*-service/src/dto/`, never entities; mapping lives in `src/mappers/` as `From` impls. | arch-qube "DTO/Entity Separation" |
| R7 | Every new service, repository, DAO and controller has unit tests in its `#[cfg(test)] mod tests` that run without a database (in-memory trait fakes, as `new-feature.sh` generates). | arch-qube "Test Coverage Threshold"; SonarQube quality gate (Jenkins stage "SonarQube Analysis") |
| R8 | Clippy `all` + `pedantic` must be clean with `-D warnings`. Where a lint is wrong for one item, use `#[allow(clippy::<lint>, reason = "...")]` on that item; never relax `[workspace.lints]`. | Jenkins stage "Unit Tests" (`docker-compose.test.yml`) |
| R9 | SQL is a string literal with `?` binds (SQLx rejects `format!` SQL at compile time). Schema changes are a new file in `migrations/`, never an edit of an applied one. | the compiler; migrations (on you) |
| R10 | The Rust version comes from `rust-toolchain.toml`; the Dockerfiles use the same tag. Change both in one PR. | CI images (on you) |

## 4. Before you say "done"

Run these, in this order, and paste the last line of each into your summary:

```bash
tools/arch-check.sh                                       # same image, --framework rust, --threshold 90 as CI; must print "arch-check: PASS"
cargo test --workspace --lib --bins --no-fail-fast        # what the CI test lane runs
cargo clippy --workspace --lib --bins -- -D warnings      # the CI clippy gate (pedantic via [workspace.lints])
```

Without a local toolchain, run the identical lane in Docker:
`docker compose -f docker-compose.test.yml build test && docker compose -f docker-compose.test.yml run --rm test`.

`arch-check.sh` exiting 2 means "could not check" (no Docker), which is not a pass. A rule
listed as "Not evaluated" in `arch-qube-reports/arch-qube.md` for a layer you touched means
not done. Do not lower `--threshold`, add exclusions or `#[allow]` without a reason to get a
pass — fix the structure. In a product made by `new-app.sh`, `arch-qube.yaml` maps the
renamed `<name>-rest` / `<name>-grpc` / `<name>-core` crates to their layers; deleting it makes
the REST crate scan as unlayered and the gate fail.

## 5. Reference

- Optional detail: `arcana-rust-developer-skill`
  (https://github.com/jrjohn/arcana-skills/tree/main/arcana-rust-developer-skill).
  This file is the source of truth: if the skill and this file disagree, this file wins.
