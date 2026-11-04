# SpecTcl compiled surface — performance plan

**Status: plan.** The measured baseline and the implementation plan for
making a `SpecTcl`-authored command surface cost what a compiled-in one
costs at query time, and nothing at all when no open document needs it.
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

The CLI agrees on the direction: `tcl diag` over autosetup's `lib/` takes
35 s under `jim` and 74 s under `tcl8.6`. Analysing Jim code as Tcl 8.6
costs twice as much as analysing it as Jim. The per-lookup numbers above
rule out the registry query as the cause; see *Open question* below.

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
- **Fixed in passing.** `spec_for_this_registry` never applies the
  roster, so a `jim` document admits `coroutine` through
  `has_command_in_this_dialect`. The surface makes one answer for both.

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
| 1 | `evaluator_generation` stops building a host. The host builds lazily per pack on its first slot. UPF hooks lower to native descriptors. `could_collide` uses a bitmap. The trace env var becomes a `OnceLock`. The reload early-out on an unchanged key. One snapshot memo for every caller. | `surface_costs`: no host build on a thread that runs no pack hook | −200–450 ms off the first request on every new worker thread for every non-UPF document. −22% and −21% of the CLI floor. |
| 2 | Lazy packs: manifests, triggers, per-profile overlays, per-environment `OverlayEpoch`. | A `tcl8.6` workspace evaluates no EDA pack: RSS and CPU at initialize. An EDA document still resolves its vendor commands on first open. | −1.9 s CPU and −99 MiB at startup in a workspace with no EDA document. The first-tokens path is clear of pack work. |
| 3 | `SpecArena`, pre-parsed `VersionKey`s, `CompiledSurface`, per-document `Arc` handle, cached derived name sets. | Differential surface ≡ `resolve_spec` test. A per-lookup budget in `surface_costs`. | ~20–30 ns per lookup. No version parsing on query paths. Registry selection is an atomic load. |
| 4 | Hook lowering pass, input inference, eager tabulation, the single warm hook VM with per-pack child interps, sandbox by construction, process-wide L2 cache and quarantine. | `hostile.tclspec` containment unchanged. A hook microbench (empty body, resolver, folder; cold, L1, L2, table). | No hook warm-up anywhere after publish. One VM's memory instead of threads × packs. Most calls answered without the VM. |
| 5 | Optional persistent evaluated snapshot. | Cache ≡ fresh evaluation. | Workspace-pack load off the startup path entirely. |

## Open question: why `tcl8.6` analyses Jim code at half the speed of `jim`

The per-lookup costs rule out registry resolution. A callgrind pair of
`tcl diag --dialect {jim,tcl8.6} lib/help.tcl` is the next measurement.
The suspects are the unknown-head paths a Tcl 8.6 reading of Jim code
reaches more often: W002's cross-dialect search for a dialect that has the
head, and the version-gate diagnostics. Its result decides whether a phase
above absorbs the fix or the general latency work does.
