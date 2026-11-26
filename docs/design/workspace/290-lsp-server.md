# 290: split tcl-lsp-server

Status: designed 26 November 2026. Plan item 290. Reopens the decision in
[build-test-cost](../../work/build-test-cost/current-state.md) ("Decided
against": splitting `tcl-lsp-server`). Builds on [289](289-lsp-core.md) (the
feature crates) and item 286 (`tcl-db`). [295](295-hub-crates.md) builds on
it.

## Problem

`tcl-lsp-server` is 120k lines in `src`: 70k production, 50k in test files
and modules, plus 56k in `tests/`. `lib.rs` is 1,359 lines; the 73k-line
`lib.rs` the old decision described is gone. `Backend` is
`{ state: Arc<BackendState>, analysis_cooperation }` (`lib.rs:522`), and
`BackendState` (`lib.rs:537–1111`) has 89 fields. 409 methods sit in
`impl Backend` blocks in 120 files across 22 top-level modules.

The old decision held that a module split buys nothing (one crate is one
compile unit) and a crate split means untangling `Backend`. Both are still
true, but the goal has changed: an edit to hover handling rebuilds the whole
server, and the server's cone is 202 crates (deduplicated, normal), 35 of
them workspace crates. It links every domain crate for a handful of
commands:

| Dependency | Used in |
|---|---|
| `tcl-bigip` | `commands/bigip_rules.rs`, `commands/bigip_objects.rs`, `protocol/lifts.rs`, `diagnostics/f5_models.rs`, `navigation/references.rs`, `document_features/requests.rs` |
| `tcl-bigip-query` | `commands/bigip_objects.rs` |
| `f5-xc` | `commands/xc_translate.rs`, `diagnostics/report.rs` |
| `tcl-explorer`, `tcl-diagram` | `commands/document_views.rs` |
| `tcl-spectcl` | 15 files (pack reload, notices, diagnostics delivery, the VFS) |
| `salsa` | 21 files, though the database is `tcl-lsp-db`'s |

`tcl-spectcl` links `tcl-spec-hooks`, `tcl-engine-tclvm`, `tcl-vm` and
`tcl-codegen`, so the language server links the VM.

### How tangled `Backend` is

Measured over production code (26 November 2026):

- **Methods.** Of the `self.method()` calls between modules, 74 cross from
  one top-level module to another. The targets are few: `dialects` (dialect
  resolution, 22 methods), `analysis` (getting an analysis), the transform
  ledger (`document_transform`), and `readonly_reference_reports`.
- **Fields.** 41 of the 89 fields are touched by more than two modules. The
  shared core is small: `client` (10 modules), `documents` (14), `store`
  (16), `db` (17), `db_files` (13), `workspace_index` (14),
  `analyser_inputs_gate` (14), `class_factory_generation` (11). Most others
  belong to one module: `configuration` owns about 30 settings fields,
  `source_publication` the `live_publication_*` and `db_tombstones` fields,
  `diagnostics` the `diag_*` and `closed_diag_*` fields, `semantic_tokens`
  its four `semantic_tokens_*` fields.

So the state splits by owner, and a small set of services is shared.

## Design

Inherent `impl Backend` blocks cannot cross crates. The split replaces them
with a session type that owns the state in parts, and handler crates of free
functions over it. Ranks are in brackets.

| Crate | Takes | Prod |
|---|---|---|
| `tcl-lsp-session` [1] | `BackendState` as `Session`; `dialects`, `analysis`, `configuration` (the settings model and reload), `tracked_locks`, `rt`, `analysis_cooperation`, `fresh_analysis_cache`, `diagnostic_read_cancellation`, `environment_notice`, `path_glob`, `vfs`; `uri_norm::DocumentPath` and `protocol`'s position lifts (`lift_lsp_range`, `line_col_to_byte_offset`) | 9.3k |
| `tcl-lsp-documents` [2] | `documents`, `source_publication`, `document_transform`, `file_rename_publication_custody`, `closed_scan_input_custody`, `provider_source`, `recovery_source`, `rewrite_wire` | 9.3k |
| `tcl-lsp-workspace` [3] | `workspace`, `spec_packs`, `studio_source`, `package_source` | 8.2k |
| `tcl-lsp-diagnostics` [4] | `diagnostics`, `current_diagnostics`, `proposal_diagnostics`, `workspace_diagnostic_sources`, `policy_truth_table` | 11.9k |
| `tcl-lsp-navigation` [4] | `navigation`, `readonly_reference_reports`, `rename`, `method_families`, `code_actions`, `semantic_tokens`, `document_features` | 24.9k |
| `tcl-lsp-commands` [5] | `commands` (BIG-IP, XC, explorer and diagram views, fix-all, catalogue queries) | 3.2k |
| `tcl-lsp-server` [6] | `language_server` (`impl LanguageServer`), `protocol` (capabilities, request plumbing), `service`, `stdio_pump`, `exit_watchdog`, `transport_liveness`, `uri_norm`, `main.rs` | 2.6k |

`tcl-lsp-navigation` is the largest at 25k. It holds every read-only request,
which share `readonly_reference_reports`' publication transactions (rename
calls them 6 times, navigation 4). It splits further if item 259's
measurement shows hover and rename edits worth isolating.

### Interfaces

```rust
// tcl-lsp-session
pub struct Session {
    pub client: Arc<dyn ClientSink>,
    pub documents: Documents,        // documents, store, edit_order, transforms
    pub settings: Settings,          // the ~30 configuration fields
    pub dialects: DialectState,      // default, session, folder and document overrides
    pub workspace: WorkspaceState,   // index, package resolver, scan gates, rehoming
    pub packs: PackState,            // spec packs, studio sources, reload sequence
    pub db: DbState,                 // tcl-db handle, files, project, tombstones, gates
    pub diagnostics: DiagnosticState,
    pub tokens: TokenState,
    pub plugins: Plugins,            // 289's contributors, set by the entry point
}
impl Session {
    pub fn dialect_for(&self, uri: &Uri) -> ResolvedDialect;
    pub async fn analysis(&self, uri: &Uri) -> Result<Arc<DocumentAnalysis>, Cancelled>;
    pub fn registry_for(&self, uri: &Uri) -> Arc<CommandRegistry>;   // packs included
}
pub trait ClientSink: Send + Sync {          // what handlers send to the editor
    fn publish_diagnostics(&self, uri: Uri, diags: Vec<Diagnostic>, version: Option<i32>);
    fn refresh_semantic_tokens(&self);
    fn log(&self, level: MessageType, text: String);
    async fn apply_edit(&self, edit: WorkspaceEdit) -> Result<bool, ClientError>;
}
pub trait CommandHandler: Send + Sync {
    fn names(&self) -> &'static [&'static str];
    async fn execute(&self, session: &Session, name: &str, args: Vec<Value>)
        -> Result<Option<Value>, ServeError>;
}

// tcl-lsp-navigation (each handler crate alike)
pub async fn hover(session: &Session, params: HoverParams) -> Result<Option<Hover>, ServeError>;
```

- Handler crates depend on `ls-types` (the protocol types), not on
  `tower-lsp-server`. `ServeError` maps to `jsonrpc::Error` in the transport.
- `ClientSink` hides `tower_lsp_server::Client` (named in 10 modules today).
  The transport implements it; handler tests use an in-memory sink.
- `CommandHandler` lets `tcl-lsp-commands` register its commands. The
  transport routes `workspace/executeCommand` by name. A build without
  BIG-IP tooling leaves the BIG-IP handler out.
- The `Backend` type stays, in the transport crate, as
  `Backend { session: Arc<Session>, cooperation }`, so the WASM transports'
  uses (`tcl_lsp_server::Backend`, `service`, `rt`, `analysis_cooperation`,
  `uri_norm`, `vfs`) keep their paths or move to `tcl_lsp_session` re-exported
  once from `tcl_lsp_server`. That single re-export is the only one allowed:
  the transport is what the facades are for.

### Back-edges to clear

With the ranks above, the cross-crate references that point the wrong way
(`crate::` paths and `self.` calls, production code):

| From → to | Count | Fix |
|---|---|---|
| handlers → transport (`uri_norm::DocumentPath` 20, `protocol::lift_lsp_range` 5, `bounded_client_request` 3, `line_col_to_byte_offset` 2) | 42 | move to `tcl-lsp-session` |
| `analysis`, `configuration` (session) → `spec_packs`, `workspace`, `documents` | 8 | the current-registry lookup and pack-path settings move into `PackState` in the session; the two document lookups become `Documents` methods |
| `workspace` → `diagnostics`, `navigation` | 8 | a diagnostics reschedule and a reference refresh become session events (`Session::invalidate`) that the higher crates subscribe to |
| `documents` → `workspace` | 1 | `uri_under_folder` moves to the session |
| `navigation` → `diagnostics` | 2 | the shared diagnostic-subject lookup moves to `tcl-ide-diagnostics` |

### Domain code and the database

- `tcl-lsp-commands` is the only server crate that depends on `tcl-bigip`,
  `tcl-bigip-query`, `f5-xc`, `tcl-explorer` and `tcl-diagram`. The four
  other files that name `tcl-bigip` (`protocol/lifts.rs`,
  `diagnostics/f5_models.rs`, `navigation/references.rs`,
  `document_features/requests.rs`) go through 289's adapter (`tcl-ide-f5`)
  instead. `diagnostics/report.rs`'s `f5-xc` use becomes a `FindingSource`.
- `salsa` is named only in `tcl-lsp-session` and `tcl-db` (overview rule 7).
  The 21 files that name it today use `salsa::Setter` on inputs, `Cancelled`
  and the database handle; they go through `DbState`'s methods.
- `tcl-spectcl` is named by `tcl-lsp-workspace` (pack reload, discovery) and
  `tcl-lsp-diagnostics` (pack notices). The session holds the active pack set
  as data (`PackState`), so `tcl-lsp-navigation` does not link the loader.

### Decisions

- **A session type with owned parts, not traits per handler group.** The
  fields already cluster by owner; a struct of parts keeps borrow and lock
  structure as it is (each field keeps "its original locks and ownership",
  as `lib.rs:534` says) and makes the move mechanical.
- **Free functions, not extension traits.** A trait per crate on `Session`
  would read like today's methods but hide which crate a call goes to.
- **Transport on top, thin.** Everything that knows `tower-lsp-server`,
  tokio's runtime and stdio is one 2.6k crate, so the WASM and WASI
  transports share the rest unchanged.

### Open questions

- **Does `Session` stay `Arc<Session>` with interior locks, or move to
  per-part `Arc`s?** Measured by the lock-contention benchmark in
  `tracked_locks` and the LSP latency suite; change only if a part's lock is
  shown to serialise unrelated requests.
- **Do handler crates need tokio?** 29 modules name `tokio::` today, mostly
  `sync`. If they need only `sync` primitives, the handler crates take
  `tokio` with `default-features = false, features = ["sync"]`, which builds
  on every wasm target. Settled by `rg 'tokio::' --type rust` after the move.

## Work list

1. Introduce `ClientSink` in place of direct `Client` use; implement it for
   `Client` (still one crate).
2. Regroup `BackendState`'s fields into the part structs (still one crate);
   `Backend` derefs to `Session`.
3. Clear the back-edges in the table.
4. Turn each module's `impl Backend` methods into free functions over
   `&Session`, one module group per change.
5. Introduce `CommandHandler`; move `commands` behind it.
6. Extract `tcl-lsp-session`, then the documents, workspace, diagnostics,
   navigation and commands crates, then reduce `tcl-lsp-server` to the
   transport. Rewrite every consumer; no re-export but the one above.
7. Split the tests: each handler crate's tests run against `Session` with an
   in-memory `ClientSink`; tests that drive JSON-RPC stay in the transport
   crate (`tests/`, 56k lines, is mostly these).
8. Regenerate `tcl-lsp-server-wasm`'s and `tcl-lsp-server-wasi`'s lockfiles
   and build both (`make lsp-server-wasm`, `make lsp-server-wasi`).

## Verification

Done when the seven crates build, the rank gate passes, and:

- no crate but `tcl-lsp-server` names `tower_lsp_server`;
- no crate but `tcl-lsp-session` (and `tcl-db`) names `salsa`;
- no crate but `tcl-lsp-commands` depends on `tcl-bigip`, `f5-xc`,
  `tcl-explorer` or `tcl-diagram`;
- the LSP end-to-end partitions, `npm run test:web` and the WASI host test
  pass.

Record:

- rebuild count and time for an edit to `navigation/hover.rs` (today the
  whole server; target `tcl-lsp-navigation` and the transport);
- each crate's cone; the transport's must not exceed today's 202;
- request latency on the LSP benchmark (the `lsp-client` skill), release
  build, before and after: free functions and the extra indirection through
  `ClientSink` must not show.

## Risks

- **Concurrent edits.** Agents edit this crate daily. Each step is one
  mechanical change landed quickly, after other work stops at a clean point.
- **Lock order.** Regrouping fields must not change which lock is taken
  first. `tracked_locks` records order; its checks run on every step.
- **WASM.** The browser build compiles every handler crate for
  `wasm32-unknown-unknown`. A handler crate that picks up a native-only
  dependency breaks only the browser build, which CI runs in a separate job.
  The rank gate's per-crate target check (no `tokio` runtime features below
  the transport) catches it earlier.

## Corrections to the overview

- Build-test-cost's "73k-line `lib.rs`" is history; the plan item already
  says 1,359 lines. Production code is 70k of the 120k.
- The plan item's "120 files" is verified (120 files contain
  `impl Backend`), with 409 methods in them.
- The plan item puts the handler groups "over `tcl-db` and `tcl-lsp-core`".
  After 289 they sit over `tcl-lsp-session`, which holds `tcl-db`, and over
  the `tcl-ide-*` crates each group needs.
