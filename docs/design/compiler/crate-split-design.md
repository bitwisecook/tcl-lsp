# Splitting tcl-compiler into layered crates

Status: proposal, 26 November 2026; tracked as part of plan item 77. Refines the "Crates" table in
[architecture-tidy-plan](../../work/name-handling/architecture-tidy-plan.md#crates)
and answers the blocker recorded in
[build-test-cost §4](../../work/build-test-cost/current-state.md): "any further
split first needs cycles broken by design".

## Where we are

- `tcl-compiler` is 910k lines in `src/`: 564k production and 346k test, plus
  68k in `tests/`. It has 121 top-level modules and 25 consuming crates.
- 96 of those modules form one strongly connected graph. Every edit therefore
  rebuilds the whole crate as one compile unit, and then everything above it.
- The lib tests pull `tcl-spectcl`, `tcl-vm` and `tcl-engine-tclvm` in as
  dev-dependencies, and `tcl-spectcl` depends on `tcl-lsp-core`. So after an edit
  to `tcl-compiler`, `cargo test -p tcl-compiler --lib` rebuilds `tcl-lsp-core`,
  `tcl-spec-hooks` and `tcl-spectcl` before the test binary links. This
  dependency chain is the "9 crates compiled" seen in every failset log.
  - 35 test files need those crates: 13 in `src` and 22 for `tcl_vm`, plus 8 in
    `tests/`.

The cycles come from a few shared vocabularies that live in the wrong module,
not from algorithms that truly need each other:

| Edge (production refs) | What crosses it | Fix |
|---|---|---|
| `ir` → `command_binding` (54) | `SourceNamespaceKey`, `CommandAllocationSite`, `BindingKind`, `SourceInvocationBinding` | identity and key types move down to `tcl-names` |
| `command_binding` → `signature_scan` (317) | `SignatureSourceNameInput`, `SignatureNamespaceScope`, `SignatureSourceNameKey` | name types move down to `tcl-names` |
| `command_binding` → `analyser` (~280) | `ResolvedAnalysisInput` (150), `AnalysisResult` (47), `Analyser::new` (33), `ProcDef`, `ClassDef` | inputs and fact types move down to `tcl-model`; running a body analysis becomes a trait that binding defines and the analyser implements |
| `registry_invocation` ↔ `command_binding` (270 / 793) | `OriginalInvocationSourcePurpose`, `SourceCommandBindings`, `SourceInvocationBinding` | they stay in one crate at first, and `tcl-invocation` is split off once the shared types have moved |
| `cfg_builder` → `optimiser` (8), `command_binding` → `optimiser` (3), `compilation_unit` → `taint` (3) | stray helpers | move each helper to the lower side |

## Principles

1. **Each crate owns its data types.** A lower crate never names a higher one.
   Where a lower layer needs behaviour from above (binding asking for a body
   report), the lower crate defines a trait. The higher crate implements it, and
   the session injects the implementation.
2. **Vocabulary below algorithms.** Identities, keys, fact records, lattices and
   diagnostics form small, stable crates. They rarely change, so editing an
   algorithm never rebuilds them.
3. **The API is narrow and explicit.** Each crate exposes a top-level `api`
   module, or curated `pub use` items, instead of `pub mod` everything. The
   roughly 150 `#[doc(hidden)] pub` items made for codegen become named API, or
   move.
4. **The registry rule is unchanged.** Per-command knowledge stays in
   `tcl-registry`, and `registry-axes` covers every new crate.
5. **No façade.** A crate that re-exports everything keeps every consumer
   depending on every layer, which undoes the split. Each extraction rewrites its
   consumers to the new crate in the same change (most of the ~6.5k
   `tcl_compiler::` paths map one-to-one), and `retired_api_gate` forbids the old
   path from then on. What remains of `tcl-compiler` is the unit crate, and it
   re-exports nothing.

## Proposed crates (bottom up)

Below this stack sit the crates that already exist: `tcl-lexer`, `tcl-syntax`,
`tcl-expr`, `tcl-registry`, `tcl-dialect`, `tcl-core-types`, `tcl-runtime-api`
and `tcl-cmd-core`.

| Crate | Takes | Size (prod) | Who uses it directly |
|---|---|---|---|
| `tcl-source` | `parsing` (CST), `segmenter`, `word_subst`, `text`, source maps | ~6k | `tcl-sslictcl`, `tcl-spectcl`, `tcl-irule-test`, segmentation users |
| `tcl-names` | identity and key types: `SourceNamespaceKey`, `CommandAllocationSite`, `BindingKind`, `Place`, signature name and scope types (the item 77 identity layer) | ~8k | everything above |
| `tcl-model` | fact vocabulary with no algorithms: `ProcDef`, `ClassDef`, analysis inputs and settings, one `Diagnostic`/`DiagCode` (today there are two), `Answer`/`Knowledge`, lattices (`bounded_set`, `LatticeValue`, completion codes), side-effect records | ~15k | LSP, MCP, CLI and DB read results through it |
| `tcl-ir` | IR and CFG data types: `Module`, `Procedure`, `Script`, `Statement`, `CommandTokens`, `cfg` types | ~8k | `bpf-tcl-ir`, `tcl-explorer`, `tcl-diagram`, `f5-xc` |
| `tcl-binding` | `command_binding`, `registry_invocation`, `realm`, `raw_binding`, `variable_bindings`, `var_resolve`, `signature_scan` (the plan folds its walk into binding), `unit_scope`, `dynamic_names` | ~235k | `tcl-lsp-core` name features, `tcl-irules` |
| `tcl-lowering` | `lowering`, `lowering_hooks`, `ir_helpers`, `cfg_builder`, SSA family (`ssa`, `memory_ssa`, `state_ssa`, `effect_ssa`, `world_state_ssa`) | ~45k | `tcl-codegen`, `tcl-vm` tests, `tcl-debugger` |
| `tcl-dataflow` | `sccp`, `value_transfer`, `tcl_expr_eval`, `type_infer`, `shimmer`, intervals, `gvn`, `interprocedural`, `var_escape`, `taint`, `static_loops`, `side_effects` | ~85k | `tcl-explorer`, `tcl-codegen` |
| `tcl-analyser` | `analyser`, `compiler_checks`, `irules_checks`; implements binding's body-report trait | ~115k | LSP stack, MCP, CLI |
| `tcl-optimiser` | `optimiser`, `semantic_optimisation` | ~22k | `tcl-spec-hooks`, `tcl-engine-tclvm`, CLI |
| `tcl-native-plan` | `executable_ir`, `native_lowering`, `common_aot_plan`, `target_contract`, `codegen_helpers`, `backend_registry` | ~20k | `tcl-codegen`, `bpf-tcl-ir` |
| `tcl-compiler` (unit crate; possibly renamed `tcl-unit`) | `compilation_unit`, `source_graph`, `auto_path_eval`; no re-exports | ~15k | whole-unit consumers (`CompilationUnit`) |

```
tcl-lexer / tcl-syntax / tcl-registry / tcl-dialect / tcl-runtime-api
        │
   tcl-source ── tcl-names ── tcl-model ── tcl-ir
                     │
                 tcl-binding  (defines BodyReporter trait)
                     │
                 tcl-lowering
                 ┌───┴────────────┐
           tcl-dataflow     tcl-native-plan ── tcl-codegen, bpf-tcl-ir
           ┌───┴───────┐
     tcl-analyser  tcl-optimiser
           └─────┬─────┘
           tcl-compiler (unit: CompilationUnit, source_graph; no re-exports)
```

`tcl-binding` remains the largest crate. Once its shared types have moved, the
next cut is `tcl-invocation`, the registry adapter (`registry_invocation`, 39k),
placed below binding.

### What each consumer gains

- **`bpf-tcl-ir`** builds against `tcl-ir` and `tcl-native-plan` only, with no
  binding or analyser.
- **`tcl-sslictcl`** needs only `tcl-source`.
- **`tcl-codegen`** stops rebuilding when the analyser or optimiser changes.
- **The LSP crates** depend on `tcl-analyser`, `tcl-model` and, for whole-unit
  work, the unit crate.
- **An edit to the analyser** (its 92k test lines make it the busiest area)
  rebuilds the analyser and what sits above it, not binding, lowering or
  dataflow.
- **The other crates compile in parallel.** Today one frontend unit serialises
  the whole crate.

## Sequence

Each step is its own change, keeps every gate green, and records the build-time
delta.

0. **Quick win, independent of the split.** Move the 35 test files that need
   `tcl-spectcl`, `tcl-vm` or `tcl-engine-tclvm` into a new `tcl-compiler-it`
   crate, and drop those dev-dependencies. A separate crate is needed because
   cargo builds dev-dependencies for every test target, including `--lib`. This
   also removes the `tcl-spectcl` ↔ `tcl-compiler` dev cycle. Expected effect:
   `cargo test -p tcl-compiler --lib` rebuilds one crate after an edit, not nine.
1. **Gate before moving anything.** Add an xtask `module-layers` manifest that
   gives each module its target crate. It records the count of back-edges
   (`use crate::X` from a lower to a higher layer) and may only shrink, in the
   same ratchet style as `walkers.tsv`. Cycles can then be broken in place while
   the code is still one crate.
2. **Move the vocabulary.** Move the `tcl-names` and `tcl-model` types (pure
   moves, with re-exports at the old paths), and merge the two `Diagnostic`
   types.
3. **Invert binding → analyser.** Binding defines a `BodyReporter` trait, which
   `tcl-analyser` implements, and binding's body-analysis cache becomes keyed by
   the trait.
4. **Extract the near-acyclic crates.** `tcl-source` (`segmenter` ↔ `parsing` is
   only 7/3 refs and stays inside one crate) and `tcl-ir`.
5. **Extract the rest as each back-edge count reaches 0.** In order:
   `tcl-binding`, `tcl-lowering`, `tcl-dataflow`, `tcl-native-plan`,
   `tcl-analyser`, `tcl-optimiser`.
6. **Gates follow the crates.**
   - Add a `deny.toml` `wrappers` list per new crate (crate-layer).
   - Re-point the roots of `walker-ownership`, `registry-axes`,
     `owner-resolution`, `segmentation-drift` and `parser-ownership`, as the
     codegen split did.
7. **Façade retired.** `tcl-compiler` keeps only the unit code and re-exports
   nothing. This step is tracked in plan item 77.

## Risks

- **Generic and `#[inline]` code** gets instantiated in downstream crates. Watch
  the release binary size and the WASM build per step. Release builds already
  use LTO.
- **Widened visibility.** Every `pub(crate)` that crosses a new boundary becomes
  `pub`. Keep a crate-level `api` module and mark the rest `#[doc(hidden)]`, and
  make the ratchet count it.
- **Caching.** The salsa database (see [caching](../../work/name-handling/caching-design.md))
  lives in the unit crate or `tcl-lsp-db`, and the lower crates expose pure functions
  over inputs.
