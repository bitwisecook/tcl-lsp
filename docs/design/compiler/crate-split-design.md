# Splitting tcl-compiler into layered crates

Status: designed 26 November 2026. Tracked as [plan item 90](../../work/name-handling/plan.md),
which the working order puts ahead of every other stage, with items 117 and 118
alongside. This is the concrete form of the "Crates" table in
[architecture-tidy-plan](../../work/name-handling/architecture-tidy-plan.md#crates).
It answers the blocker recorded in [build-test-cost §4](../../work/build-test-cost/current-state.md):
"any further split first needs cycles broken by design".

## Goal

A clean rearchitecture of the workspace into many small shared crates. Each has
a clean interface and supports every consumer that needs it: the LSP, MCP, the
CLI, the VM, codegen, the eBPF back end, the explorer, the SpecTcl tooling and
the BIG-IP tools. The aims are fast builds, little caching, and good runtime
performance. `tcl-compiler` is the first and largest piece; `tcl-registry`
(items 117 and 118) is the second. On this branch the tree may stay broken for
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
  - Items 117 and 118, and later splitting the registry's per-command data from
    its lookup code, keep the registry out of cones that only need the lookup
    vocabulary.
- **Dev-dependency hygiene.** A crate's tests may not depend on a crate above
  it, because that puts the whole upper stack into every worktree that tests
  it. Step 0 removes today's case. Once the split is done, the `module-layers`
  gate enforces this for dev-dependencies too.
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

Item 259 measures this before the split, and then again as each crate builds.
The measure covers four representative isolated edits:
- a lexer or syntax change;
- a registry command spec;
- an analyser rule;
- an LSP feature.

For each edit it records the crates compiled, wall time, peak RSS and
target-directory growth.

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
  - Only 35 test files need those crates: 13 for `tcl_spectcl` and 22 for
    `tcl_vm` in `src`, plus 8 in `tests/`.
- **`tcl-registry` sits under everything.** It is 583k lines, and the registry
  rule sends every per-command fix there. A registry edit rebuilds `tcl-compiler`
  and the whole stack above it.
  - Item 117 takes the 58k lines of `native_*` algorithms out, and with them the
    `value_transfer` and `resolved_invocation` evaluation (35k).
  - Item 118 takes the 176k lines of BIG-IP schema data out.

The linker is not the problem. Measured 26 November 2026 on the 183 MB
`tcl-compiler` test binary, a link takes 1.4–1.7 s with the bundled `rust-lld`
(already the default), 1.2 s with `mold` and 6.5 s with GNU ld. Switching to
mold would save about 0.4 s per link.

## Where the cycles come from

The cycles come from shared vocabularies kept in the wrong module, not from
algorithms that need each other:

| Edge (production refs) | What crosses it | Fix |
|---|---|---|
| `ir` → `command_binding` (54) | `SourceNamespaceKey`, `CommandAllocationSite`, `BindingKind`, `SourceInvocationBinding` | identity and key types move down to `tcl-names` |
| `command_binding` → `signature_scan` (317) | `SignatureSourceNameInput`, `SignatureNamespaceScope`, `SignatureSourceNameKey` | name types move down to `tcl-names` |
| `command_binding` → `analyser` (~280) | `ResolvedAnalysisInput` (150), `AnalysisResult` (47), `Analyser::new` (33), `ProcDef`, `ClassDef` | the inputs and fact types move down to `tcl-model`; running a body analysis becomes a trait that binding defines and `tcl-diagnostics` implements |
| `registry_invocation` ↔ `command_binding` (270 / 793) | `OriginalInvocationSourcePurpose`, `SourceCommandBindings`, `SourceInvocationBinding` | one crate at first; `tcl-invocation` is split off once the shared types have moved |
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
same. `tcl-names`, `tcl-model`, `tcl-lowering` and `tcl-native-plan` are
additions: they are the cuts that break the cycles above. The tidy plan's
`tcl-ir` is split in two, data types (`tcl-ir`) and builders (`tcl-lowering`),
because binding reads IR types while lowering reads binding.

| Crate | Takes | Size (prod) | Direct users | Plan items that land here |
|---|---|---|---|---|
| `tcl-source` | `parsing` (CST), `segmenter`, `word_subst`, `text`, source maps | ~6k | `tcl-sslictcl`, `tcl-spectcl`, `tcl-irule-test` | 82 (structured words), 41 (one segmentation) |
| `tcl-names` | identity and key types: `SourceNamespaceKey`, `CommandAllocationSite`, `BindingKind`, `Place`, `CommandRef`, signature name and scope types | ~8k | every crate above | 77 (identity layer), 66 (frame value) |
| `tcl-model` | fact vocabulary with no algorithms: `ProcDef`, `ClassDef`, analysis inputs and settings, one `Diagnostic`/`DiagCode` (there are two today), `Answer`/`Knowledge`, the lattices (`bounded_set`, `LatticeValue`, completion codes), side-effect records | ~15k | LSP, MCP, CLI, DB | 78 (answer types), 50–53 (typed IR facts) |
| `tcl-ir` | IR and CFG data types: `Module`, `Procedure`, `Script`, `Statement`, `CommandTokens`, `cfg` types | ~8k | `bpf-tcl-ir`, `tcl-explorer`, `tcl-diagram`, `f5-xc` | 50–53 |
| `tcl-binding` | `command_binding`, `registry_invocation`, `realm`, `raw_binding`, `variable_bindings`, `var_resolve`, `signature_scan`, `unit_scope`, `dynamic_names` | ~235k | LSP name features, `tcl-irules` | 84 (one binding walk), 19–25 (lookup), 245 |
| `tcl-lowering` | `lowering`, `lowering_hooks`, `ir_helpers`, `cfg_builder`, SSA family (`ssa`, `memory_ssa`, `state_ssa`, `effect_ssa`, `world_state_ssa`) | ~45k | `tcl-codegen`, `tcl-vm` tests, `tcl-debugger` | 83 (every body a unit) |
| `tcl-analysis` | `sccp`, `value_transfer`, `tcl_expr_eval`, `type_infer`, `shimmer`, intervals, `gvn`, `interprocedural`, `var_escape`, `taint`, `static_loops`, `side_effects` | ~85k | `tcl-explorer`, `tcl-codegen` | 81 (kernels), 59–65 (constants) |
| `tcl-native-plan` | `executable_ir`, `native_lowering`, `common_aot_plan`, `target_contract`, `codegen_helpers`, `backend_registry` | ~20k | `tcl-codegen`, `bpf-tcl-ir` | |
| `tcl-diagnostics` | `analyser`, `compiler_checks`, `irules_checks`; implements binding's body-report trait | ~115k | LSP stack, MCP, CLI | 85 turns it into rules over `tcl-semantic` |
| `tcl-optimiser` | `optimiser`, `semantic_optimisation` | ~22k | `tcl-spec-hooks`, `tcl-engine-tclvm`, CLI | 244 |
| `tcl-compiler` (the unit crate, possibly renamed `tcl-unit`) | `compilation_unit`, `source_graph`, `auto_path_eval`; no re-exports | ~15k | whole-unit consumers (`CompilationUnit`) | 90 step 7 (salsa) |
| `tcl-semantic` (later) | the query API and fact store | new | every feature | 85 (phase 6) |

```
tcl-lexer / tcl-syntax / tcl-registry (+ 117, 118 data crates) / tcl-dialect / tcl-runtime-api
        │
   tcl-source ── tcl-names ── tcl-model ── tcl-ir
                     │
                 tcl-binding  (defines the body-report trait)
                     │
                 tcl-lowering
                 ┌───┴────────────┐
           tcl-analysis     tcl-native-plan ── tcl-codegen, bpf-tcl-ir
           ┌───┴──────────┐
   tcl-diagnostics   tcl-optimiser          (later: tcl-semantic over binding + analysis)
           └─────┬────────┘
           tcl-compiler (unit crate: CompilationUnit, source_graph; no re-exports)
```

`tcl-binding` is still the largest crate. Once its shared types have moved, the
next cut is `tcl-invocation`: the registry adapter (`registry_invocation`, 39k),
placed below binding.

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
[`crate-split/`](crate-split/). Breakage between steps is allowed:

- **Where keeping things building costs work, skip it.** If keeping a step
  compiling would take shims, temporary re-exports or duplicated types, the
  step moves the code directly and fixes the edges afterwards.
- **Measure at milestones, not every step.** The gates, the failset
  comparisons and the build-time and size measurements run when each crate
  first builds again, not after every move.
- **Steps 0 and 1 are still green changes.** They are small, and step 0 pays
  for itself on every edit that follows.

0. **Test crate.** Move the 35 test files that need `tcl-spectcl`, `tcl-vm` or
   `tcl-engine-tclvm` into a new `tcl-compiler-it` crate, and drop those
   dev-dependencies.
   - This also removes the `tcl-spectcl` ↔ `tcl-compiler` dev cycle.
   - Expected effect: after an edit, `cargo test -p tcl-compiler --lib`
     rebuilds one crate, not nine.
1. **Map before moving.** Add a `module-layers` xtask manifest that assigns
   each module its target crate. It reports the back-edges (`use crate::X`
   from a lower layer to a higher one) per pair and per target crate.
   - The report is the work list for steps 2–5.
   - Once the split is done it becomes the gate that keeps the layers clean.
2. **Move the vocabulary.** Move the `tcl-names` and `tcl-model` types.
   - Consumers are rewritten in the same change, with no re-exports at the old
     paths.
   - The two `Diagnostic` types are merged into one.
3. **Invert binding → analyser.** Binding defines the body-report trait,
   `tcl-diagnostics` implements it, and binding's body-analysis cache is keyed
   through the trait.
4. **Extract the near-acyclic crates:** `tcl-source` and `tcl-ir`. The
   `segmenter` ↔ `parsing` cycle is only 7/3 refs and stays inside `tcl-source`.
5. **Extract the rest as each back-edge count reaches 0.** In order:
   `tcl-binding`, `tcl-lowering`, `tcl-analysis`, `tcl-native-plan`,
   `tcl-diagnostics`, `tcl-optimiser`. Every extraction also does three things:
   - moves its consumers off `tcl-compiler`;
   - adds a `deny.toml` `wrappers` list for the new crate (the crate-layer
     gate);
   - re-points the roots of `walker-ownership`, `registry-axes`,
     `owner-resolution`, `segmentation-drift`, `parser-ownership` and the
     owner-crate column of the shared-utility contracts, as the codegen split
     did.
6. **Façade retired.** `tcl-compiler` keeps only the unit code and re-exports
   nothing.
7. **Caching.** Salsa over the split crates (see [caching](../../work/name-handling/caching-design.md)).
   - The database lives in the unit crate or `tcl-lsp-db`, and the lower crates
     expose pure functions over their inputs.
   - Items 101 and 103 add the binding and workspace tiers.

Items 117 and 118 run alongside steps 0–6. They change crates below
`tcl-compiler` only, so they don't conflict with the split.

## Docs and paths

Design documents cite source paths such as `rust/tcl-compiler/src/...`. Each
extraction rewrites the paths of the modules it moves, in the same change; the
documents that describe the crate layout ([project-layout](../contracts/project-layout.md),
[current architecture](../rust/current-architecture.md), [engineering guide](../rust/engineering-guide.md))
are updated in the step that changes the layout.

## Risks

- **Generic and `#[inline]` code** is instantiated in downstream crates. Watch
  the release binary size and the WASM build at each step. Release builds
  already use LTO.
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
  Each crate exposes pure functions, and the one salsa layer caches only at the
  unit boundaries where re-computation has measured cost (see
  [caching](../../work/name-handling/caching-design.md)). Every other hand-written
  memo is deleted.
