# Splitting tcl-compiler into layered crates

Status: designed 26 November 2026. Tracked as [plan item 90](../../work/name-handling/plan.md),
which the working order puts ahead of every other stage, with items 117 and 118
alongside. Revised 26 November 2026 after an independent review: binding's
layout against lowering is undecided pending item 278, the value kernel gets
its own crate, and the steps are finer. This is the concrete form of the "Crates" table in
[architecture-tidy-plan](../../work/name-handling/architecture-tidy-plan.md#crates).
It answers the blocker recorded in [build-test-cost §4](../../work/build-test-cost/current-state.md):
"any further split first needs cycles broken by design".

## Goal

A clean rearchitecture of the workspace into many small shared crates. Each has
a clean interface and supports every consumer that needs it: the LSP, MCP, the
CLI, the VM, codegen, the eBPF back end, the explorer, the SpecTcl tooling and
the BIG-IP tools. The aims are fast builds, little caching, and good runtime
performance. `tcl-compiler` is the first and largest piece. `tcl-registry` is the second:
it becomes a core plus one crate per dialect profile and per package
([registry split](../registry/registry-split-design.md), item 261), and items 117
and 118 move its native algorithms and BIG-IP data out. On this branch the tree may stay broken for
a long time while the layout changes. The order is chosen for the least total
work and the cleanest result, not to keep every step green.

### Parallel worktrees

The acceptance test for the layout: many independent worktrees work in
parallel, each on an isolated part of the stack, and none fills the disk or
memory. Each worktree's build compiles only the dependency cone of what it
touches. Five things decide that:

- **Cone size.** Working on a crate compiles its dependencies, and testing it
  compiles its dev-dependencies too. Today building `tcl-registry` alone
  compiles 54 crates, `tcl-compiler` 80 and `tcl-lsp-core` 179. Every Tcl-facing
  crate sits above the 583k-line registry.
  - The split makes cones small.
  - The [registry split](../registry/registry-split-design.md) (item 261) keeps
    per-command data out of every generic crate's cone. Generic crates depend
    on the registry core only, and only entry points select surfaces. Items 117
    and 118 shrink the core further.
- **Dev-dependency hygiene.** A crate's tests may not depend on a crate above
  it, because that puts the whole upper stack into every worktree that tests
  it. Step 0 removes today's cases, which are not only `tcl-compiler`'s:
  `tcl-registry` tests through `tcl-spectcl`; `tcl-lsp-core` through
  `tcl-spectcl` and `tcl-codegen`; `tcl-lsp-db` through `tcl-vm`,
  `tcl-spectcl` and `tcl-codegen`; and `tcl-vm` through `tcl-compiler` and
  `tcl-codegen` (items 246 and 269–272). The `module-layers` gate (item 247)
  checks dev-dependency edges from the start, with these cases as a known list
  that may only shrink.
- **Peak memory.** rustc's peak memory grows with the size of the crate it is
  compiling. A 564k-line crate is one large process; ten smaller crates are ten
  smaller ones, which `-j` can bound.
- **Disk per worktree.** Debuginfo is already off and the target directory is
  per checkout (`scripts/dev/agent-build-env.sh`). What remains is incremental
  state (9.7 GB today, mostly `tcl-compiler`'s three caches) and large test
  binaries (183 MB for `tcl-compiler`'s lib tests). Both shrink with cone and
  crate size.
- **Sharing across worktrees.** Third-party dependencies and unchanged
  workspace crates are rebuilt in every worktree's target directory. A shared
  compiler cache would let worktrees reuse each other's outputs, and a cache
  keyed on absolute paths would miss across checkouts. That is item 260.
  sccache does not cache incrementally compiled crates, so the edited crate's
  cone is still rebuilt per worktree. Item 260 sets a target for that share and
  a target-directory pruning policy.
- **Build variants.** The same crate compiled more than once in one worktree:
  with and without a `test-support` feature (item 296), and under both the
  dev and test profiles (item 297). Item 259 records crates compiled per
  profile and per feature set, test-run peak RSS and rust-analyzer memory per
  worktree.

Item 259 measures this before the split, and then again as each crate builds.
The measure covers four representative isolated edits:
- a lexer or syntax change;
- a registry command spec;
- an analyser rule;
- an LSP feature.

For each edit it records the crates compiled, wall time, peak RSS and
target-directory growth.

Runtime performance is measured beside build cost. Each crate's first green
build records analysis latency on the analysis-performance corpus and the VM
benchmarks (`scripts/perf/interp/`), before and after (item 90's measurement
record). These run on release builds: the agents' opt-level-1 test builds lose
cross-crate inlining without LTO, so latency measured there would charge the
split for a cost release builds do not have.

## Why now

Build and iteration time is paid on every change in every later stage of the
plan, and three facts dominate it:

- **`tcl-compiler` is one compile unit.** It is 910k lines in `src/` (564k
  production, 346k test) plus 68k in `tests/`. 96 of its 121 top-level modules
  form one strongly connected graph. Any edit rebuilds the whole crate, then the
  9 to 25 crates above it.
- **Its lib tests pull in the upper stack.** They take `tcl-spectcl`, `tcl-vm`
  and `tcl-engine-tclvm` as dev-dependencies, and `tcl-spectcl` depends on
  `tcl-lsp-core`.
  - Cargo builds dev-dependencies for `--lib` test targets too. So after an edit,
    `cargo test -p tcl-compiler --lib` rebuilds `tcl-lsp-core`,
    `tcl-spec-hooks` and `tcl-spectcl` before its test binary links.
  - That is the 9 crates and 2 to 6 minutes seen in every failset log.
  - Only 12 `src` test blocks need `tcl-spectcl`, plus 7 `tests/` files and
    the `value_transfers` bench that use `tcl-spectcl` or `tcl-engine-tclvm`
    ([step 0](crate-split/00-test-dependencies.md)). `tcl-vm` does not depend
    on `tcl-compiler`, so its 22 users stay.
- **`tcl-registry` sits under everything.** It is 583k lines, and the registry
  rule sends every per-command fix there. A registry edit rebuilds `tcl-compiler`
  and the whole stack above it.
  - Item 117 takes the 58k lines of `native_*` algorithms out, and with them the
    `value_transfer` and `resolved_invocation` evaluation (35k).
  - Item 118 takes the 176k lines of BIG-IP schema data out.

Smaller levers, each a plan item with its own measurement:

- `test-support` features build a crate and its dependents twice in one
  worktree (item 296);
- check and clippy run under the dev profile and tests under the test profile
  at opt-level 1, so a worktree compiles every crate twice (item 297);
- `xtask` builds `tcl-vm` and `tcl-runtime` for every gate run, for one gate
  (item 298);
- `tcl-version`'s build script re-runs on every commit, and `tcl-dialect`'s
  generates into `OUT_DIR` under 43 crates (item 299);
- about 99k lines of generated tables are compiled as Rust and could be
  embedded as data (item 300, beside item 119's generators).

The linker is not the problem. Measured 26 November 2026 on the 183 MB
`tcl-compiler` test binary, a link takes 1.4–1.7 s with the bundled `rust-lld`
(already the default), 1.2 s with `mold` and 6.5 s with GNU ld. Switching to
mold would save about 0.4 s per link.

## Where the cycles come from

Most cycles come from shared vocabularies kept in the wrong module, not from
algorithms that need each other. One does not: binding and value analysis are
one fixpoint, so binding calls into lowering, SSA and the value kernel by
design. Counts are production references in the binding modules (`rg`, test
files excluded, 26 November 2026):

| Edge (production refs) | What crosses it | Fix |
|---|---|---|
| `ir` → `command_binding` (54) | `SourceNamespaceKey`, `CommandAllocationSite`, `BindingKind`, `SourceInvocationBinding` | identity and key types move down to `tcl-names` |
| `command_binding` → `signature_scan` (317) | `SignatureSourceNameInput`, `SignatureNamespaceScope`, `SignatureSourceNameKey` | name types move down to `tcl-names` |
| `command_binding` → `analyser` (~280) | `ResolvedAnalysisInput` (150), `AnalysisResult` (47), `Analyser::new` (33), `ProcDef`, `ClassDef` | the inputs and fact types move down to `tcl-model`; running a body analysis becomes a trait that binding defines and `tcl-diagnostics` implements |
| `registry_invocation` ↔ `command_binding` (270 / 793) | `OriginalInvocationSourcePurpose`, `SourceCommandBindings`, `SourceInvocationBinding` | one crate at first; `tcl-invocation` is split off once the shared types have moved |
| binding → lowering and SSA: `ir_helpers` (115), `lowering` (68), `ssa` (52), `cfg_builder` (18), `lattice_rebase` (18) | binding and value analysis are one fixpoint per body unit ([design integration](../../work/name-handling/design-integration.md#decisions), decision 1) | undecided: inversion or co-location, chosen by item 278 |
| binding → value analysis: `tcl_expr_eval` (37), `value_transfer` (7), `interprocedural` (6), and a few into `sccp`, `gvn`, `side_effects`, `taint` | the value kernel and transfer functions | the kernel moves below binding to `tcl-values` (item 280); the rest is part of item 278's inventory |
| `cfg_builder` → `optimiser` (8), `command_binding` → `optimiser` (3), `compilation_unit` → `taint` (3) | stray helpers | each helper moves to the lower side |

## Principles

1. **Each crate owns its data types.** A lower crate never names a higher one.
   Where a lower layer needs behaviour from above, such as binding asking for a
   body report, the lower crate defines a trait. The higher crate implements it,
   and the unit crate wires the two together.
2. **Vocabulary sits below algorithms.** Identities, keys, fact records,
   lattices and diagnostics live in small, stable crates, so editing an algorithm
   never rebuilds them.
3. **Each crate's API is narrow and explicit.** Each crate exposes a top-level
   `api` module, or curated `pub use` items of its own, rather than making every
   module `pub`. The roughly 150 `#[doc(hidden)] pub` items made for codegen
   either become named API or move.
4. **The registry rule is unchanged.** Per-command knowledge stays in
   `tcl-registry` (or its data crates after items 117 and 118), and
   `registry-axes` covers every new crate.
5. **No façade.** A crate that re-exports everything keeps every consumer
   depending on every layer, which undoes the split.
   - Each extraction rewrites its consumers to the new crate in the same change.
     Most of the ~6.5k `tcl_compiler::` paths map one-to-one.
   - From then on, `retired_api_gate` forbids the old path.
   - What remains of `tcl-compiler` is the unit crate, which re-exports nothing.

## Crates (bottom up)

The crates that already exist sit below this stack: `tcl-lexer`, `tcl-syntax`,
`tcl-expr`, `tcl-registry`, `tcl-dialect`, `tcl-core-types`, `tcl-runtime-api`
and `tcl-cmd-core`. The names follow the tidy plan where the meaning is the
same. `tcl-names`, `tcl-values`, `tcl-model`, `tcl-lowering` and
`tcl-native-plan` are additions: they are the cuts that break the cycles above.
The tidy plan's `tcl-ir` is split in two, data types (`tcl-ir`) and builders
(`tcl-lowering`), because binding reads IR types.

**Undecided, pending item 278:** whether `tcl-binding` sits below
`tcl-lowering` and `tcl-analysis`. Binding also calls lowering, SSA and value
analysis, because the two run as one fixpoint per body unit. Item 278
inventories those edges and chooses between inversion (binding exposes its
transfer functions, and a fixpoint driver above runs both) and co-location
(binding, lowering and SSA in one crate). The table and diagram below show
the inversion layout; under co-location the `tcl-binding` and `tcl-lowering`
rows become one crate.

| Crate | Takes | Size (prod) | Direct users | Plan items that land here |
|---|---|---|---|---|
| `tcl-source` | `parsing` (CST), `segmenter`, `word_subst`, `text`, source maps | ~6k | `tcl-sslictcl`, `tcl-spectcl`, `tcl-irule-test` | 82 (structured words), 41 (one segmentation) |
| `tcl-names` | identity and key types: `SourceNamespaceKey`, `CommandAllocationSite`, `BindingKind`, `Place`, `CommandRef`, signature name and scope types | ~8k | every crate above | 77 (identity layer), 66 (frame value) |
| `tcl-values` | item 81's single value kernel: the value lattice (`LatticeValue`), abstract values, the `tcl_expr_eval` core | new | `tcl-model`, `tcl-binding`, `tcl-analysis`, the optimiser | 81 (kernels), 280 |
| `tcl-model` | fact vocabulary with no algorithms: `ProcDef`, `ClassDef`, analysis inputs and settings, one `Diagnostic`/`DiagCode` (there are two today), `Answer`/`Knowledge`, the result lattices (`bounded_set`, completion codes), side-effect records | ~15k | LSP, MCP, CLI, DB | 78 (answer types), 50–53 (typed IR facts) |
| `tcl-ir` | IR and CFG data types: `Module`, `Procedure`, `Script`, `Statement`, `CommandTokens`, `cfg` types | ~8k | `bpf-tcl-ir`, `tcl-explorer`, `tcl-diagram`, `f5-xc` | 50–53 |
| `tcl-binding` (layout against `tcl-lowering` undecided, item 278) | `command_binding`, `registry_invocation`, `realm`, `raw_binding`, `variable_bindings`, `var_resolve`, `signature_scan`, `unit_scope`, `dynamic_names` | ~235k | LSP name features, `tcl-irules` | 84 (one binding walk), 19–25 (lookup), 245 |
| `tcl-lowering` (or one crate with binding, item 278) | `lowering`, `lowering_hooks`, `ir_helpers`, `cfg_builder`, SSA family (`ssa`, `memory_ssa`, `state_ssa`, `effect_ssa`, `world_state_ssa`) | ~45k | `tcl-codegen`, the VM's compiler tests, `tcl-debugger` | 83 (every body a unit) |
| `tcl-analysis` | `sccp`, `value_transfer`, the rest of `tcl_expr_eval`, `type_infer`, `shimmer`, intervals, `gvn`, `interprocedural`, `var_escape`, `taint`, `static_loops`, `side_effects` | ~85k | `tcl-explorer`, `tcl-codegen` | 59–65 (constants), 117's evaluation half |
| `tcl-native-plan` | `executable_ir`, `native_lowering`, `common_aot_plan`, `target_contract`, `codegen_helpers`, `backend_registry` | ~20k | `tcl-codegen`, `bpf-tcl-ir` | |
| `tcl-diagnostics` | `analyser`, `compiler_checks`, `irules_checks` (or `irules_checks` in `tcl-irules`, item 285); implements binding's body-report trait | ~115k | LSP stack, MCP, CLI | 85 turns it into rules over `tcl-semantic` |
| `tcl-optimiser` | `optimiser`, `semantic_optimisation` | ~22k | `tcl-spec-hooks`, `tcl-engine-tclvm`, CLI | 244 |
| `tcl-compiler` (the unit crate, possibly renamed `tcl-unit`, item 258) | `compilation_unit`, `source_graph`, `auto_path_eval`; no re-exports | ~15k | whole-unit consumers (`CompilationUnit`) | |
| `tcl-db` (today's `tcl-lsp-db`, renamed by item 286) | the one database, above the unit crate; no `tcl-lsp-core` dependency | ~19k | `tcl-lsp-server`, `tcl-mcp`, the CLI | 90's step 14; later the tiers of 101, 103, 46–49, 73 |
| `tcl-semantic` (later) | the query API and fact store | new | every feature | 85 (phase 6) |

```
tcl-lexer / tcl-syntax / tcl-registry core / tcl-dialect / tcl-runtime-api   (dialect, library and catalogue crates: entry points only)
        │
   tcl-source ── tcl-names ── tcl-values ── tcl-model ── tcl-ir
                                  │
       ┌──────────────────────────┴───────────────────────┐
       │ tcl-binding  (defines the body-report trait)     │  undecided, item 278:
       │      │                                           │  inversion (as drawn, with a
       │ tcl-lowering (SSA family)                        │  fixpoint driver above both) or
       └──────────────────────────┬───────────────────────┘  co-location (one crate)
                 ┌────────────────┴───┐
           tcl-analysis         tcl-native-plan ── tcl-codegen, bpf-tcl-ir
           ┌───┴──────────┐
   tcl-diagnostics   tcl-optimiser          (later: tcl-semantic over binding + analysis)
           └─────┬────────┘
           tcl-compiler (unit crate: CompilationUnit, source_graph; no re-exports)
                 │
              tcl-db (the one database) ── tcl-lsp-server, tcl-mcp, CLI
```

`tcl-binding` is still the largest crate. Once its shared types have moved, the
next cut is `tcl-invocation`: the registry adapter (`registry_invocation`, 39k),
placed below binding if the ledger allows it (item 282).

`SourceInvocationBinding` crosses the `ir` → `command_binding` edge. Its home
is open: `tcl-names` if it is a key, otherwise `tcl-ir` as the annotated
command. Items 248 and 252 record the answer.

### What each consumer gains

- **`bpf-tcl-ir`** builds against `tcl-ir` and `tcl-native-plan` only, with no
  binding or diagnostics.
- **`tcl-sslictcl`** needs only `tcl-source`.
- **`tcl-codegen`** stops rebuilding when the diagnostics or the optimiser
  change.
- **The LSP crates** depend on `tcl-diagnostics` and `tcl-model`, plus the unit
  crate for whole-unit work.
- **An edit to the analyser**, the busiest area with 92k test lines, rebuilds
  `tcl-diagnostics` and what sits above it, not binding, lowering or analysis.
- **The other crates compile in parallel.** Today a single frontend unit
  serialises the whole crate.

## Sequence

The step numbers are item 90's, and each step has its own design in
[`crate-split/`](crate-split/). The plan items are not numbered in step order,
so each step names its items in the order they run. Breakage between steps is
allowed: this is the "`work` branch exception" that the
[engineering guide](../rust/engineering-guide.md) and the
[tidy plan](../../work/name-handling/architecture-tidy-plan.md#phases) point to.

- **Where keeping things building costs work, skip it.** If keeping a step
  compiling would take shims, temporary re-exports or duplicated types, the
  step moves the code directly and fixes the edges afterwards.
- **Measure at milestones, not every step.** The gates, the failset
  comparisons, the build-time and size measurements and the runtime
  measurement record (analysis latency and VM benchmarks on release builds) run
  when each crate first builds again, not after every move.
- **Steps 0 and 1 are still green changes.** They are small, and step 0 pays
  for itself on every edit that follows.

0. **Test dependencies.** Take the upper stack out of each crate's tests.
   - 246, `tcl-compiler` ([design](crate-split/00-test-dependencies.md)), in
     three parts: 266 moves the 7 `tests/` files and the bench into a new
     `tcl-compiler-it` crate; 267 moves the 12 `src` test blocks behind
     `test-support` and drops the `tcl-spectcl` and `tcl-engine-tclvm`
     dev-dependencies; 268 re-keys the CI shards and the gates. This also
     removes the `tcl-spectcl` ↔ `tcl-compiler` dev cycle. Expected effect:
     after an edit, `cargo test -p tcl-compiler --lib` rebuilds one crate, not
     nine.
   - 269 `tcl-registry`, 270 `tcl-lsp-core`, 271 `tcl-lsp-db` and 272
     `tcl-vm`: the other verified dev-dependency cascades.
1. **Map before moving.** 247 adds the `module-layers` gate, which assigns
   each module its target crate and reports the back-edges (`use crate::X`
   from a lower layer to a higher one) per pair and per target crate. It checks
   dev-dependency edges from the start. 273 seeds and reviews the ledger.
   - The report is the work list for steps 2–12.
   - From step 13 on it becomes the gate that keeps the layers clean.
2. **`tcl-source` first** (251, then 281). It is near-acyclic (the `segmenter`
   ↔ `parsing` cycle is only 7/3 refs and stays inside it), it has rank 1, and
   the `tcl-names` types may carry source ranges. 281 rewires the SpecTcl
   loader and `tcl-sslictcl` onto it.
3. **`tcl-names`** (248). The identity and key types move unchanged; 248 also
   records where `SourceInvocationBinding` goes. Item 77's redesign continues
   inside the new crate as 274, alongside the later steps.
4. **`tcl-values`** (280). Item 81's value kernel moves below binding, before
   binding is extracted and before the rest of the analysis move, because
   binding calls `tcl_expr_eval`.
5. **`tcl-model`** (249 result lattices, 275 `ProcDef`/`ClassDef` and the
   inputs, 276 the result and side-effect records with `Answer`, 277 one
   `Diagnostic` type, checked by golden diffs of LSP, CLI and MCP output).
   Consumers are rewritten in the same change, with no re-exports at the old
   paths.
6. **`tcl-ir`** (252). It takes `SourceInvocationBinding` as the annotated
   command if 248 found it is not a key.
7. **Binding against lowering and analysis** (278, 250, 279). 278 inventories
   every binding → SSA, lowering, SCCP and value edge and chooses inversion or
   co-location; it is a design step and can start once step 1 lands. 250
   inverts binding → analyser: binding defines the body-report trait,
   `tcl-diagnostics` implements it, and binding's body-analysis cache is keyed
   through the trait. 279 applies 278's choice.
8. **`tcl-binding`** (253, then 282). 253 is a pure move; folding
   `signature_scan` into the one binding walk stays item 84's work. 282 places
   `tcl-invocation`.
9. **`tcl-lowering`** (254), empty if 278 chose co-location.
10. **`tcl-analysis`** (255). Item 117's evaluation half lands after it.
11. **`tcl-native-plan`** (256, then 283, which turns the `#[doc(hidden)] pub`
    items into named API).
12. **`tcl-diagnostics` and `tcl-optimiser`** (257 `tcl-diagnostics` alone,
    284 `tcl-optimiser`, 285 whether `irules_checks` belongs to `tcl-irules`).
13. **The unit crate and the database** (258, 286, 287). 258 retires the
    façade and decides the unit crate's name. 286 renames `tcl-lsp-db` to
    `tcl-db`, the one database at the top, and removes its `tcl-lsp-core`
    dependency. 287 re-points `tcl-compiler-it` onto the split crates.
14. **Caching.** The caches the split leaves are deleted, or moved into
    `tcl-db` where re-computation at a unit boundary has measured cost (see
    [caching](../../work/name-handling/caching-design.md)). The lower crates
    expose pure functions over their inputs. The per-body tiers (items 101
    and 103, and 46–49 and 73) come later, after items 83 and 84.

Every extraction (steps 2–12) also does three things:

- moves its consumers off `tcl-compiler`;
- adds a `deny.toml` `wrappers` list for the new crate (the crate-layer gate);
- re-points the roots of `walker-ownership`, `registry-axes`,
  `owner-resolution`, `segmentation-drift`, `parser-ownership` and the
  owner-crate column of the shared-utility contracts, as the codegen split
  did.

Items 117 and 118 run alongside steps 0–13, and they do conflict with the
split in places:

- Item 117 moves `value_transfer` and `resolved_invocation` evaluation out of
  the registry into the analysis layer. Until step 10 that layer is inside
  `tcl-compiler`'s cycle, so this evaluation half lands after item 255, into
  `tcl-analysis` or `tcl-values`. Its native-model half changes crates below
  `tcl-compiler` only and can run at once.
- Both change `tcl-registry`, which item 261's registry split is also
  reshaping. Their order against it is being redesigned with the
  [registry split](../registry/registry-split-design.md).

## The rest of the workspace

`tcl-compiler` and the registry are the two largest pieces, but the same goal
applies to every crate. Each item below is small and has its own "done when".
Sizes are `wc -l` over each crate's `src` (26 November 2026).

| Crate | Size | Problem | Item |
|---|---|---|---|
| `tcl-spectcl` | 43k | depends on `tcl-lsp-core` (`vfs`, `code_actions`, one `LspRange`), an upward edge [project-layout](../contracts/project-layout.md) rule 4 allows and the tidy plan forbids | 288 retires the rule's exception |
| `tcl-lsp-core` | 245k | links the domain crates `tcl-bigip` (58k), `tcl-irules` (5k) and `tcl-sslictcl` (9k) instead of reaching them through interfaces | 289 |
| `tcl-lsp-server` | 120k | one crate; `Backend` has `impl` blocks in 120 files. [build-test-cost](../../work/build-test-cost/current-state.md) decided against splitting it; reopened under this goal | 290 |
| `tcl-vm` | 220k | depends on `tcl-registry` for the native algorithms; after item 117, on the native-model crate | 291 |
| `tcl-codegen` | 69k | depends on the whole of `tcl-compiler` | 292 |
| `runtime/rust` (`tcl-runtime`) | 308k | the second engine, duplicating the VM's frame and namespace code | the existing engine-convergence items: 88 and its shared items (20, 72, 161, 171, 178, 186, 203), deferred on this branch |
| `tcl-mcp` | 12k | depends on `tcl-cli`, `f5-cli` and `f5-xc` for five functions | 293 |
| `tcl-explorer` | 13k | depends on `tcl-vm`, `tcl-codegen` and `tcl-spectcl` | 294 |
| hub crates: `tcl-dialect` (32k), `tcl-lexer` (27k), `tcl-syntax` (48k), `tcl-core-types` (4k) | | 30–43 dependent crates each, so an edit rebuilds most of the workspace | 295 |

## Docs and paths

Design documents cite source paths such as `rust/tcl-compiler/src/...`. Each
extraction rewrites the paths of the modules it moves, in the same change; the
documents that describe the crate layout ([project-layout](../contracts/project-layout.md),
[current architecture](../rust/current-architecture.md), [engineering guide](../rust/engineering-guide.md))
are updated in the step that changes the layout.

## Risks

- **Generic and `#[inline]` code** is instantiated in downstream crates. Watch
  the release binary size and the WASM build at each step. Release builds
  already use LTO; the opt-level-1 test builds do not, so they lose
  cross-crate inlining, and analysis latency is measured on release builds
  only.
- **Widened visibility.** Every `pub(crate)` item that crosses a new boundary
  becomes `pub`.
  - Keep a crate-level `api` module and mark the rest `#[doc(hidden)]`.
  - The ratchet counts these items.
- **Work in flight.** Items being worked on while modules move will conflict.
  Agents finish their current item at a clean point, and new item work waits
  for the step that moves its modules.
- **A long broken period.** While crates don't build, the test suites and the
  conformance runner can't vouch for behaviour. Each crate's first green build
  is followed by a failset comparison against the last green commit before the
  move. Item 75's conformance runner is re-run once the unit crate builds.
- **Caching kept small.** The split removes caches rather than adding them.
  Each crate exposes pure functions, and the one database at the top
  (`tcl-db`) caches only at the unit boundaries where re-computation has
  measured cost (see [caching](../../work/name-handling/caching-design.md)).
  Every other hand-written memo is deleted. Identity types do not use salsa
  interning, so `tcl-names` and the registry stay salsa-free and the VM can
  use them.
