# SpecTcl compiled surface — performance plan

**Status: phases 1, 2, 4 and 5 implemented; phase 3 in part.** The measured
baseline, what was built, and what it measured, for making a
`SpecTcl`-authored command surface cost what a compiled-in one costs at
query time, and nothing at all when no open document needs it.
The priorities, in order:

1. **Editor latency**: time to first semantic tokens, then edit → tokens.
2. **Running memory**.
3. **Load time**.

Companions: [spec-packs.md](spec-packs.md) (the loader and hook contract this
restructures), [../rust/lsp-performance.md](../rust/lsp-performance.md) (the
server's latency contract).

General analysis latency is out of scope here: the multi-second
`semanticTokens/full` on some files, and the multi-GB resident set after a few
dozen open documents. It is covered separately. Where the baseline below
shows it, it is recorded so that it is not mistaken for a `SpecTcl` cost.

## Results

Same machine and harnesses as the baseline below (`ci` profile, 4 vCPU).

| measurement | baseline | now |
|---|--:|--:|
| `tcl diag --source 'set a 1'`, `tcl8.6` / `jim` | 2.6 s, 180 MiB | 0.42 s / 0.32 s, 90 MiB |
| autosetup `lib/` (`tcl diag`), `jim` / `tcl8.6` | 35 s / 74 s | 16.7 s / 40.1 s |
| bundled packs at startup, no EDA document open | 1,862 ms, 99 MiB | 136 ms, 35 MiB (manifests) |
| first EDA document's packs (Xilinx + SDC + UPF) | part of the above | 421 ms, on demand, off the request |
| hook VM build per worker thread | 200–450 ms, 15 MiB | 0.2–0.3 ms (a proxy) |
| UPF `define_power_model` resolver, first / warm call | a VM bootstrap / ~28 µs | 0.02 ms / 69 ns, no VM ever started |
| `resolve_spec`, known head, `tcl8.6` / `jim` | 863 / 494 ns | 235 / 183 ns |
| `resolve_spec`, unknown head | 266 ns | 20 ns |
| SpecTcl rendering of the Tcl 8.6 surface, evaluated | 3,635 ms | 697 ms (native `build_default`: 1.3 ms) |

What landed, by phase:

1. **Waste.**
   - Registry semantic keys are interned. Their equality had walked the
     whole command table on every invocation-metadata query: a quarter of
     all analysis instructions.
   - `evaluator_generation` no longer builds a VM host. The server's eager
     per-worker host build is gone.
   - UPF's 16 option-arity bodies are now data
     (`OptionArity::OptionalOneOf`, `OptionalNotOption`).
   - Each spec's lowered surface declarations are memoised process-wide.
   - The deferred-write trace variable is read once.
   - Collision admission is computed once per load.
   - Include-free evaluations are memoised across reloads and consumers.
   - Binding a pack body's original source words is no longer quadratic in
     the body's size. That alone was half of a cold process's instructions.
2. **Lazy packs.** Covered in [spec-packs.md](spec-packs.md) § *Lazy
   bundled packs*. A pack set that installs no command still installs its
   overlay, which fixed a latent analysis-context miss.
3. **Surface (partial).**
   - Authored windows answer from a per-thread memo, so a row's version
     comparison is a hash lookup after its first ask.
   - Version parsing no longer allocates.
   - The whole-surface resolved-name table is not built. Per-lookup
     resolution measures at about 1% of analysis instructions now, below
     its cost in cache invalidation across a hundred call sites.
4. **Hooks.** Covered in [spec-packs.md](spec-packs.md) § *Where bodies
   run*: one warm hook service per process, and tabulation of count-only
   resolvers.
5. **Precompiled.**
   - `cargo xtask gen-bundled-manifests` compiles into the server binary,
     and so into every editor extension: which bundled packs may defer,
     the packages each needs, and the bundled count-only resolvers' answer
     tables.
   - `cargo xtask gen-equivalence-packs` renders the native Tcl 8.6 and
     iRules surfaces as SpecTcl under `rust/tcl-spectcl/equivalence/`. These
     are like-for-like measuring instruments that no product loads. The
     only native command the renderings cannot carry is iRules' `when`.

In the language server (`dialect_pair_bench.py`, autosetup `lib/`, 18
files, opened without waiting for the scan):

| | `jim` | `tcl8.6` |
|---|--:|--:|
| first tokens | 294 ms | 757 ms |
| open → tokens, p50 / p95 | 0.58 s / 3.4 s | 1.4 s / 12.0 s |
| edit → tokens, p50 | 2.4 s | 30.6 s |
| peak RSS | 2.4 GB | 3.0 GB |

On a five-line file, edit → tokens after 12 s idle is 77–103 ms: a request
landing on a fresh pool thread no longer rebuilds a hook VM. The remaining
latency and memory, and the `tcl8.6` reading's cost on Jim code, are
general analysis (§ *Why `tcl8.6` analyses Jim code at half the speed of
`jim`*), not SpecTcl.

The same commands cost 1.3 ms to index natively and 697 ms to evaluate as
SpecTcl. Both end as the same `CommandSpec` rows, so query cost is
identical. The remaining gap is load time only: per-command spec
construction, not parsing.

## How to measure

| tool | what it isolates |
|---|---|
| `cargo run --profile ci -p tcl-spectcl --example surface_costs` | Per phase, in-process: Jim seed, bundled pack load, hook host build per thread, registry selection, per-lookup resolve (known and unknown heads) under `jim` and `tcl8.6` |
| `python3 scripts/perf/dialect_pair_bench.py --server … --corpus DIR --dialect jim --dialect tcl8.6` | One server per dialect over the same bytes: initialize, time to first tokens, per-file open → tokens and → diagnostics, edit → tokens, RSS at each stage |
| `valgrind --tool=callgrind target/ci/tcl diag --dialect D FILE` | Attribution when `perf` is unavailable |

## Corpora

Jim Tcl has no large application corpus of the kind tcllib is for C Tcl.
The three largest public bodies of Jim code are:

| corpus | upstream | what it is | Jim sources |
|---|---|---|---|
| jimtcl | `github.com/msteveb/jimtcl` | the interpreter's own test suite, examples and `tclcompat`/`stdlib` | 159 files, 38k lines |
| autosetup | `github.com/msteveb/autosetup` | the `configure` replacement that runs under `jimsh` (also vendored by SQLite and Fossil) | 29 files, 6.4k lines (`lib/`, `auto.def`s; its tcllib example is excluded, see below) |
| OpenOCD | `github.com/openocd-org/openocd`, `tcl/` | board, target and interface scripts, run by OpenOCD's embedded Jim | 1,086 `.cfg` + `.tcl`, 54k lines |

autosetup's `examples/tcllib/{sak.tcl,tcllib_version.tcl}` drive
`semanticTokens/full` past 20 s, or past a 60 s timeout, under both
dialects. That is the general latency issue, and those files are excluded
from the paired runs.

## Baseline (work branch, `ci` profile, 4 vCPU)

### Process floor: `tcl diag --source 'set a 1'`

2.6 s and 165–180 MiB, the same under every dialect. Callgrind attributes
**91% of it to SpecTcl work that `set a 1` under `tcl8.6` never uses**:

| share | where | why |
|--:|---|---|
| 69% | `bundled::load_discovered` → `pack::load_sources_unpublished_inner` | evaluates all eight EDA packs |
| ↳ 44% | `cache::evaluate_pack_including` | static evaluation itself; `loader::segment` runs 3,245 times because each command body is re-segmented |
| ↳ 22% | `pack::could_collide` → `static_document_context_for_profile` (960 calls) | a full document context is assembled per profile, per colliding name pair |
| 21% | `hooks::ensure_thread_host` | builds the UPF pack's hook VM |
| ↳ 15% | `TclVmEngine::restrict_commands` → `Vm::remove_registered_command` | the sandbox is made by tearing a full VM down, one command cleanup at a time |
| 19% (cross-cutting) | `ParsedVersion::parse_bytes_for` | 1.2 M version-string parses from `compare_versions_bytes_for`, `satisfies_range_with_policy` and friends |

### Per phase: `surface_costs`

| phase | wall | RSS growth |
|---|--:|--:|
| `core_surfaces::ensure` (Jim seed) | 13 ms | 8 MiB |
| `bundled::load_embedded` (8 EDA packs) | **1,862 ms** | **99 MiB** |
| `bundled::packs` (the process-wide set, a second evaluation) | 1,500 ms | 85 MiB |
| first `registry_for_dialect` (`tcl8.6` / `jim` / `xilinx-eda-tcl`) | 5 / 20 / 7 ms | — |
| `ensure_thread_host`, first thread | **446 ms** | 15 MiB |
| `ensure_thread_host`, each further thread | **200–220 ms** | — |

| per lookup | `tcl8.6` (2,271 names) | `jim` (144 names) |
|---|--:|--:|
| `ResolvedContext::resolve_spec`, known head | 863 ns | 494 ns |
| `ResolvedContext::resolve_spec`, unknown head | 266 ns | 159 ns |
| `ContextRegistry::resolve_command` | 1,546 ns | 602 ns |
| select a document's context | 215 ns | 215 ns |
| select the pack-overlaid store | 916 ns | 1,638 ns |

[spec-packs.md](spec-packs.md)'s 255 ms and ~4 MB for the bundled corpus
time `evaluate_pack` alone. The load path a consumer actually takes — the
merge, the collision sweep, the per-command gates — costs 7× and 25× that.

A per-lookup resolve under `jim`, the `SpecTcl`-authored surface, is no
slower than under the native `tcl8.6`. The query-time gap to close is
between both of them and one hash probe (~20 ns), not between the two.

### The paired LSP runs

autosetup (26 files, 4.8k lines), opened without waiting for the workspace
scan, as an editor does:

| | `jim` | `tcl8.6` |
|---|--:|--:|
| initialize | 59 ms | 67 ms |
| open → tokens, p50 / p95 | 0.87 s / 10.7 s | 13.2 s / 28.7 s (10 files, then a 30 s timeout) |
| edit → tokens, p50 | 9.1 s | — |
| peak RSS | 5.2 GB | 3.0 GB at the timeout |

The CLI agrees on the direction. `tcl diag` over each whole corpus, one
process per dialect, so the 2.6 s floor is paid once:

| corpus | `jim` | `tcl8.6` | peak RSS `jim` / `tcl8.6` |
|---|--:|--:|--:|
| autosetup `lib/` (6k lines) | 35 s | 74 s | 232 / 379 MiB |
| jimtcl (159 files, 38k lines) | 645 s | 968 s | 496 / 882 MiB |
| OpenOCD `tcl/` (1,120 files, 54k lines) | 285 s | 376 s | 847 / 879 MiB |

Analysing Jim code as Tcl 8.6 costs 1.3–2.1× as much as analysing it as
Jim. The per-lookup numbers above rule out the registry query as the cause;
see *Why `tcl8.6` analyses Jim code at half the speed of `jim`* below.

## What the code does today

**Load: everything is eager.**

- `initialize` seeds the Jim core surface unconditionally.
- `initialized` awaits `reload_spec_packs(Startup)`, which evaluates every
  bundled EDA pack whatever the workspace holds.
- It then installs an overlay into each of ~21 store profiles. Each
  install is a `CommandRegistry::build_default()` from scratch, plus a
  clone-and-leak of every admitted pack spec.
- Every reload re-evaluates every pack (the in-memory `SNAPSHOT` tier is
  never reached from the server path) and leaks a fresh set of specs. That
  includes a reload whose byte-derived set key is unchanged.
- The on-disk cache stores the parse, not the evaluation, so it saves
  segmentation and nothing else.

**Hooks: compiled per thread, on first use.**

- `publish` only allocates slots. Every worker thread builds its own
  `HookHost`: one `TclVmEngine` per pack, each bootstrapping a full VM and
  `build_default()`, then deleting down to the whitelist.
- Bodies compile to bytecode on first call, per thread, per engine.
- Compiler paths call `pack_hooks::evaluator_generation()`, and that builds
  the host. So any thread that compiles anything pays 200–450 ms on its
  first use, whatever the document's dialect.
- The server's `spawn_blocking` pool keeps an idle thread for 10 s, so the
  cost recurs as threads churn.
- The only shipped hooks are UPF's 17 (16 `-arity-hook` bodies, 15 of them
  byte-identical, and one `arg_role_resolver`). None declares `-inputs`,
  so none is ever cached.
- On a miss, a call marshals the words twice. It also rebuilds the context
  dict with freshly allocated keys, invokes by name, and converts a result
  nobody reads.

**Query: availability is decided per lookup.**

- `ContextRegistry::assemble` admits specs per context into `entries`. But
  the analyser and the providers resolve over the raw store with
  `ResolvedContext::resolve_spec`, which re-runs, on every call:
  - provider and roster admission;
  - `surface_admits` on each row;
  - nearness ranking for families with two core points;
  - version-window checks, which parse version strings into `Vec`s.
- Unknown heads also read an environment variable on every miss.
- `builtin_command_names` resolves every store name into a fresh
  `HashSet<String>` per analysis. Completion and the interp-visibility
  scan do the same.
- A document's registry is re-selected from its dialect string on each
  request, through a handful of mutexes and SipHash maps.

**Memory.**

- 99 MiB for EDA packs most workspaces never use.
- 15 MiB per thread-local hook host.
- A full `by_name` index clone (~261 kB) per (profile, overlay, family,
  generation).
- Leaked specs per reload and per profile, never reclaimed.

## Target: one compiled surface for native and SpecTcl

`CommandSpec` with `&'static` storage is already the common record. A
native spec is a Rust constant, a pack spec is the loader's leaked
output, and every consumer reads either through the same fields and
fn-pointer hooks. What is missing is the *compilation* step after it:
availability, rosters, version windows and hook bodies are still
interpreted per query, per thread. The plan adds that step once, for both
sources, so they leave the loader as the same resolved data.

```
native specs ──┐                                  ┌─► CompiledSurface (per environment × realm × package set)
               ├─► SpecArena (SpecId, interned) ──┤      name → SpecId, final answer, no locks, no parsing
SpecTcl packs ─┘     pre-parsed version keys      └─► HookTable per slot: Native fn | Tabulated | Vm(slot)
                                                          Vm slots served by one warm hook VM per process
```

### 1. `SpecArena` and pre-parsed versions

- A process-wide, append-only arena of `&'static CommandSpec`, addressed
  by `SpecId(u32)` and interned by content hash. A pack reload, or a
  second profile installing the same pack, reuses the existing spec
  instead of leaking a copy.
- `SpecWindow` carries packed `VersionKey`s (from / until) computed once:
  at construction for native specs (a `const` parse), at load for pack
  rows. `SurfaceQuery` carries its point pre-parsed.
- `version_in_any_window`, `satisfies_range_with_policy` and
  `compare_versions_bytes_for` drop out of every query path. That is 19%
  of the CLI floor today, and every per-row check.

### 2. `CompiledSurface`: availability decided once

For each (environment generation, overlay key, realm, required-package
set), compile one immutable table:

- `name → SpecId`, holding the final answer `resolve_spec_in_realm` gives
  today. Roster, provider, predicate, package placement, window, nearness
  and breadth are all decided at compile time, and the rooted-name
  fallback is folded in.
- `names: Arc<[&'static str]>`, sorted, so completion takes a prefix range
  with `partition_point`.
- The derived sets each analysis rebuilds today: builtin names, the known
  set segmentation recovery reads, operator heads, and the namespace
  package ownership. Each is computed once per surface.

Rules:

- **Ownership.** Surfaces live in a global map behind an `ArcSwap`-style
  pointer, keyed by a single surface epoch that moves on any environment,
  roster, core-surface or overlay change. A document holds
  `Arc<CompiledSurface>`, so a request's registry selection becomes one
  atomic load and a pointer compare.
- **Document-local packages.** `package require` changes
  `required_package_available`. The required-package set is interned to a
  `PackageSetId`; the empty set and the few sets a workspace actually uses
  are cached, and a document's surface is compiled on first sight of its
  set (~1–2 ms over a native profile).
- **API.** `ResolvedContext::resolve_spec(store, name)` answers from the
  surface whenever `store` is that surface's store. The current
  interpretation path remains only for a foreign store (explorer, tests).
- **Correctness gate.** A differential test asserts that the surface answer
  equals today's `resolve_spec_in_realm`, for every environment × realm ×
  every store name plus a rooted spelling and an unknown name. It runs in
  `rust-check`, and it must be green before the old path is retired from
  analysis.
- **To check in passing.** `CommandRegistry::spec_for_this_registry`, behind
  `has_command_in_this_dialect` and `method_dispatch_keyword`, goes through
  `get_for_surface`, which does not consult the inherited-surface roster.
  Unless the `jim` store is pre-filtered, a store-only query admits a Tcl
  head Jim's roster omits (`coroutine`). The differential test covers it,
  and the surface gives both paths one answer.

Expected: about 600–1,500 ns becomes about 20–30 ns per lookup, and
per-analysis name-set rebuilds disappear.

### 3. Hooks: native, tabulated, or one warm VM

Every hook family keeps its shipped fn-pointer type, so `run_const_fold`
and every other consumer stay unaware. Behind the thunk, each slot is
compiled at pack load to the cheapest form that is exact:

- **Native descriptor.** A load-time *hook lowering* pass pattern-matches
  the idioms authors actually write into existing or new declarative
  fields:
  - a count-only role map → the `ArgRoleCountResolver` shape;
  - an option value from a literal set (`… in {TRUE FALSE}`) or not
    dash-prefixed → a new `OptionArity` variant;
  - a constant role map → `arg_roles` data.

  All 17 shipped UPF hooks lower this way. With that, no shipped pack runs
  the VM, and the "no packs = one relaxed load" claim in
  [spec-packs.md](spec-packs.md) becomes true again.
- **Tabulated.** Compile the body with the compiler front end, then read
  which inputs it actually uses: `llength $words`, `ctx` keys, word kinds,
  word content. A body that never reads word content is shape-cacheable
  whether or not it declared `-inputs`. For each such slot, evaluate
  `nwords = 0..=16` per dialect once at publish, on the hook thread, into
  an immutable `Arc<[Answer]>` readable from any thread without a VM.
  `option-value-start` joins the shape key so option-arity hooks qualify.
- **VM.** What remains runs on the hook service below.

**One warm VM per language server.** A dedicated OS thread owns one
`tcl_vm::Vm`.

- **Isolation.** Each pack gets its own safe child interpreter in it (the
  VM already models interps: `Vm::interps`, `interp create -safe`). Packs
  share no interpreter state, so the per-pack containment rule holds,
  while the bootstrap, the builtin tables and the thread are shared.
- **Lifetime.** Bodies compile to bytecode at publish, and a syntax error
  becomes a pack notice there rather than a first-call abstention. Pack
  interps persist across generations, keyed by content hash, so a studio
  edit rebuilds one pack, not all of them. Nothing is ever rebuilt because
  a worker thread is new.
- **Sandbox by construction.** A pack interp is created with the
  whitelist registered, rather than bootstrapping everything and removing
  the rest. Today's removal is 15% of the CLI floor.
- **Calling it.** Worker threads submit `(slot, words, shape)` over a
  bounded channel and block. The hop costs a few µs, against today's
  28 µs per invocation, and most calls never make it:
  1. the per-thread shape cache (L1, kept);
  2. a process-wide sharded answer cache (L2), keyed by (plan generation,
     slot, shape or content hash), so a shape warmed on one thread is warm
     on all;
  3. the tabulated table;
  4. the VM.
- **Per-call marshalling.** Context constants are built once per slot.
  `words` and `ctx` are skipped when the body does not reference them.
  Words cross as `&[&[u8]]`, the call goes by the captured command token,
  and the unread result conversion goes.
- **Quarantine and epoch.** Quarantine is process-wide (an atomic bitset
  per slot): one runaway costs one budget, not one per thread, and moves
  the evaluator epoch once. `pack_hooks::evaluator_generation()` becomes
  a global atomic read and never builds anything.

The rejected alternatives:

- **One VM per pack, each on its own thread.** It costs a thread and a
  bootstrap per pack, for isolation the child interp already gives.
- **One VM per worker thread (today).** It pays the warm-up once per
  thread, forever, and multiplies memory by the thread count.

### 4. Lazy packs

- **Manifest.** Computed by the static driver's header pass, or read from
  the disk cache by content hash. It records:
  - the pack name and version;
  - environments, `dialect` blocks, `file_extension` rows, ambient
    packages, surface rosters and special variables;
  - command names, each with its `required_package`;
  - whether the pack has hooks.
- **What startup publishes.** Manifests only, which is what environment
  registration, extension routing, `getEffectiveConfig` and the
  cross-pack collision notices need. `could_collide` reads a precomputed
  profile × package admission bitmap instead of assembling contexts.
- **What triggers a full evaluation** (spec construction, hook lowering,
  publication to the hook service), the first time any of these happens:
  - a document resolves to an environment whose ambient packages
    intersect the pack's requirement set;
  - a document `package require`s a package the pack provides;
  - an unknown head is one the manifest's name index says a hosted pack
    declares. This keeps W120 from becoming W123.
- **Tiers.** Bundled packs are lazy. Workspace and user packs stay eager,
  because their notices belong on the pack file, but they are evaluated off
  the first-tokens path. The Jim core surface seeds on the first `jim`
  family environment.
- **Per-environment invalidation.** Overlays install per profile on
  demand, cloned from the cached overlay-0 store rather than rebuilt.
  `OverlayEpoch` becomes per environment, so loading Vivado's surface does
  not invalidate an open `tcl8.6` document's memoised analysis.
- **Reloads.** A reload compares the byte-derived set key before
  evaluating anything, and reuses the published set when it matches. The
  in-memory snapshot memo serves the server path and
  `bundled::packs()`, so no process evaluates a pack twice.

### 5. Optional: a persistent evaluated snapshot

Serialise an evaluated pack: specs, with hook pointers written as slot and
descriptor ids, plus the lowered hook tables. A warm start then
deserialises instead of evaluating. This is only worth doing if a
workspace's own packs still show up on the first-tokens path after
phase 4.

## Phases

Each phase lands with its numbers re-measured by the two harnesses above
and recorded here, and keeps the existing gates green: `golden_packs`,
`eval_loader` (static ≡ interpreted), `spec_corpus`, `registry_sweep`,
`pack_torture`, the I6 security floor.

| phase | change | gate it adds | expected effect |
|---|---|---|---|
| 1 | Interned `RegistrySemanticKey`s and a pointer fast path in `matches_registry`. `evaluator_generation` stops building a host. The host builds lazily per pack on its first slot. UPF hooks lower to native descriptors. `could_collide` uses a bitmap. The trace env var becomes a `OnceLock`. The reload early-out on an unchanged key. One snapshot memo for every caller. | `surface_costs`: no host build on a thread that runs no pack hook | −200–450 ms off the first request on every new worker thread for every non-UPF document. −22% and −21% of the CLI floor. Up to −25% of analysis instructions in both dialects. |
| 2 | Lazy packs: manifests, triggers, per-profile overlays, per-environment `OverlayEpoch`. | A `tcl8.6` workspace evaluates no EDA pack: RSS and CPU at initialize. An EDA document still resolves its vendor commands on first open. | −1.9 s CPU and −99 MiB at startup in a workspace with no EDA document. The first-tokens path is clear of pack work. |
| 3 | `SpecArena`, pre-parsed `VersionKey`s, `CompiledSurface`, per-document `Arc` handle, cached derived name sets. | Differential surface ≡ `resolve_spec` test. A per-lookup budget in `surface_costs`. | ~20–30 ns per lookup. No version parsing on query paths. Registry selection is an atomic load. |
| 4 | Hook lowering pass, input inference, eager tabulation, the single warm hook VM with per-pack child interps, sandbox by construction, process-wide L2 cache and quarantine. | `hostile.tclspec` containment unchanged. A hook microbench (empty body, resolver, folder; cold, L1, L2, table). | No hook warm-up anywhere after publish. One VM's memory instead of threads × packs. Most calls answered without the VM. |
| 5 | Optional persistent evaluated snapshot. | Cache ≡ fresh evaluation. | Workspace-pack load off the startup path entirely. |

## Work breakdown

The registry rule holds throughout: a new capability is a `CommandSpec`
field or descriptor, with its loader spelling, renderer and studio form
moving together ([command-spec-studio.md](../contracts/command-spec-studio.md)
§ *Parity with native specs*), never a consumer matching a command name.

### Phase 1: remove the per-thread and per-load waste

| change | where |
|---|---|
| Intern semantic keys at construction. Add a `ptr::eq` fast path on the stores. | `RegistrySemanticKey` (`tcl-registry/src/registry.rs`), `InvocationMetadataContext::matches_registry` (`tcl-compiler/src/registry_invocation/metadata_context.rs`) |
| Return the published generation from a global atomic, never building a host. | `pack_hooks::evaluator_generation` (`tcl-registry/src/pack_hooks.rs`) |
| Build the host per pack, on that pack's first slot, not per thread up front. Drop the eager build in the server's `with_pack_hooks`. | `hooks::ensure_thread_host` (`tcl-spectcl/src/hooks.rs`), `with_pack_hooks` (`tcl-lsp-server/src/lib.rs`) |
| Add `OptionArity::OptionalFrom(&'static [&'static str])` and `OptionArity::OptionalUnlessDashed`, with loader spellings (`-arity optional-from {TRUE FALSE}`, `-arity optional-unless-dashed`). Rewrite UPF's 16 `-arity-hook` bodies to use them, and its `define_power_model` resolver as an `arg_role_count` row. | `tcl-registry/src/hover.rs` and every `OptionArity` reader; `tcl-spectcl/src/loader.rs`; studio form; `specs/upf.tclspec` |
| Precompute a profile × required-package admission bitmap once per load. | `pack::could_collide` (`tcl-spectcl/src/pack.rs`) |
| Read `TCL_LSP_TRACE_DEFERRED_WRITES_ANY` once, into a `OnceLock<bool>`. | `ResolvedContext::resolve_spec_in_realm` (`tcl-registry/src/model/context.rs`) |
| Compare the byte-derived set key before evaluating. Route the server path and `bundled::packs` through one snapshot memo. | `bundled::load_selected_sources_in`, `cache::evaluate_pack_including`, `bundled::packs` |
| Segment each pack file once and reuse the nested words for command bodies. | `loader::body_statements` / `loader::statements` |

### Phase 2: lazy packs

| change | where |
|---|---|
| `PackManifest`: the header facts listed in *Lazy packs*, from the static driver's first pass, and cached on disk beside the parse by `EvalSnapshotKey`. | new `tcl-spectcl/src/manifest.rs`; `loader::eval::drive_static` |
| `PackSet` entries become `Lazy { manifest, source, evaluated: OnceLock<Pack> }`. The set key stays byte-derived. | `tcl-spectcl/src/pack.rs` |
| Registration, routing and collision notices read manifests. | `registration::publish_pack_set`, `publish_pack_extension_routes`, `pack::collision_notices` |
| Evaluate only the packs whose requirement set the profile admits. Build overlays by cloning the cached overlay-0 store. | `install::registry_with_packs` / `install_into`; `tcl-registry/src/cache.rs` `build_profile_store` |
| Replace the eager per-profile loop with `ensure_profile_overlay(profile)`, called where a document's dialect resolves (open, dialect change, `package require` seen by the index). | `install_pack_overlays` / `install_reload_pack_overlays` (`tcl-lsp-server/src/lib.rs`) |
| `OverlayEpoch` keyed per environment. `overlays_current` asks only about installed profiles. | `tcl-lsp-db/src/lib.rs`; `overlays_current` |
| Seed the Jim surface on the first `jim` family environment, not in `initialize`. | `core_surfaces::ensure` call sites |

### Phase 3: the compiled surface

| change | where |
|---|---|
| `VersionKey` (packed, `Ord`) with a `const` parser. `SpecWindow` and `SurfaceQuery` carry keys. Version comparison on query paths becomes integer comparison. | `tcl-dialect/src/version.rs`, `tcl-dialect/src/model/authored_surface.rs` |
| `SpecArena` / `SpecId`, interned by content hash. Loader and install go through it. | new `tcl-registry/src/arena.rs`; `loader.rs` leak site; `install_into` |
| `CompiledSurface` built at `ContextRegistry::assemble`, holding the final answers and the derived name sets. | new `tcl-registry/src/model/compiled_surface.rs`; `assembly.rs` |
| `resolve_spec*` answer from the surface for its own store. | `tcl-registry/src/model/context.rs` |
| The analyser, completion and interp visibility borrow the surface's name sets instead of rebuilding them. | `analyser/state.rs` `builtin_command_names`, `recovery.rs`, `tcl-lsp-core/src/completion.rs`, `interp_visibility.rs` |
| A document's surface `Arc` is cached under the global surface epoch. | `TclDb::registry` (`tcl-lsp-db`), `Backend::registry_for_dialect` |
| Differential test: surface ≡ `resolve_spec_in_realm` for every environment × realm × name. | `tcl-registry/tests/` |

### Phase 4: hooks

| change | where |
|---|---|
| Hook lowering: pattern → descriptor, then compiler-IR input inference (`words` / `ctx` key / word content reads), then tabulation for `nwords ≤ 16`. Each slot is classified `Native | Tabulated | Vm`, and the class is reported in `spectcl_check` and in the studio's performance badge. | new `tcl-spectcl/src/hooks/lowering.rs`; `hooks::publish` |
| `Vm::create_sandbox_interp(whitelist) -> InterpId`. `Vm::invoke_in(interp, token, &[&[u8]])`. Per-call budgets reset on the invoking interp. An isolation audit that child interps share no namespace, command or variable state, with a test per shared table. | `tcl-vm/src/embed.rs`, `tcl-vm/src/interp.rs`; `tcl-engine-api`, `tcl-engine-tclvm` |
| The hook service thread: owns the `Vm` and one interp per pack, compiles every `Vm` slot at publish, serves a bounded channel, and keeps interps across generations by content hash. | new `tcl-spec-hooks/src/service.rs` |
| L2 answer cache (sharded, process-wide), `value_start` in `ShapeKey`, a process-wide quarantine bitset, and an epoch bump per quarantine (not per thread). | `tcl-registry/src/pack_hooks.rs` |
| Marshalling: per-slot `ctx` constants, skipping unread inputs, words as bytes, invoking by token, no result conversion. | `tcl-spec-hooks/src/host.rs`, `tcl-engine-tclvm` |

If the isolation audit fails, the fallback keeps the warm, single-thread
property: one `Vm` per pack, all owned by the same service thread and all
built at publish. It costs one bootstrap per pack (~15 MiB measured)
instead of one in total. Contention on the one thread is not expected
once tabulation and L2 absorb repeated shapes. If it is measured, the
service scales to N threads, each holding a full warm set.

## Why `tcl8.6` analyses Jim code at half the speed of `jim`

Callgrind of `tcl diag --dialect {jim,tcl8.6} lib/help.tcl` (autosetup,
216 lines): 24.3 G instructions under `jim`, 47.4 G under `tcl8.6`.

- **The difference is analyser work, not the registry.** About 8 G of
  the gap is `command_binding::source_transition_advice`
  (`AdviceInvocationContext::retain_original_procedure_body`,
  `BodyWalk::regions`, `OriginalSourceTransitionSeed::prepare_for_query`),
  reached through `dispatch_one_body_argument` from `foreach` and `proc`
  bodies. A `jim` reading never reaches it. This is general analysis
  latency, not a `SpecTcl` cost.
- **A registry cost both dialects pay.** `RegistrySemanticKey::eq`
  takes 6.6 G under `tcl8.6` and 5.9 G under `jim` (a quarter of the
  whole `jim` run), all from `InvocationMetadataContext::matches_registry`.
  The key holds every command name with its spec indices, and `eq`
  short-circuits only on `Arc` identity. A context's store and the store
  it is checked against are routinely equal in content and distinct in
  allocation (the overlay store against the plain one), so every
  invocation-metadata query compares the whole command table.

  Phase 1 fixes it two ways:
  - intern semantic keys at construction (one `Arc` per distinct key, so
    equal keys are pointer-equal);
  - give `matches_registry` a `ptr::eq` fast path on the stores.

  Phase 3's surface identity is then an integer.
