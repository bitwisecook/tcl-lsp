# `tcl-codegen` after the compiler split

Status: plan item 292, the rest of the workspace beside item 90. It builds on
step 11 (items 256 and 283, `tcl-native-plan` and named API), step 6 (item
252, `tcl-ir`), steps 9–10 (items 254 and 255, `tcl-lowering` and
`tcl-analysis`) and step 13 (item 258, the unit crate), all in the
[crate-split overview](../compiler/crate-split-design.md). Item 296's
`test-support` decision ([levers](296-300-build-levers.md#296-test-support-feature-unification))
applies to its dev-dependency. Items 272 and 271's test crates and item 294's
explorer work build on it.

## Problem

`tcl-codegen` is 68,643 lines in 144 `src` files, of which 41 files and 16,521
lines are named as tests, plus 23,659 lines in 52 `tests/` files with 815
tests. Its normal dependencies are `tcl-compiler` and eight small crates
(`Cargo.toml:34–43`). Its dev-dependencies are `tcl-compiler` with
`features = ["test-support"]`, `tcl-test-support` and `tcl-vm`
(`Cargo.toml:46–49`). Its 8 direct dependents, 15 transitive, are:

- `tcl-cli`;
- `tcl-debugger`;
- `tcl-engine-tclvm`;
- `tcl-explorer`;
- `tcl-fuzz`;
- `tcl-irule-test`;
- `tcl-mcp`;
- `tcl-vm-cli`.

Item 292 says codegen will name only `tcl-native-plan`, `tcl-lowering`,
`tcl-analysis` and `tcl-ir` after step 11. The code says otherwise. Files not
named as tests reference these `tcl_compiler` modules (`rg -o`, 26 November
2026; inline `#[cfg(test)]` modules included, so the counts are upper bounds):

| Target crate (overview) | Modules and references |
|---|---|
| `tcl-ir` | `ir` 177, `cfg` 42, `expr_ast` 17 |
| `tcl-native-plan` | `native_lowering` 46, `executable_ir` 17, `native_compilation_admission` 15, `native_byte_compilation` 12, `common_aot_plan` 10, `mixed_region_plan` 5, `backend_registry` 5, `native_integer_proof` 4, `codegen_helpers` 4, `target_contract` 2 |
| `tcl-lowering` | `cfg_builder` 28, `lowering` 21, `ssa` 3, `lattice_rebase` 3, `lowering_hooks` 1 |
| `tcl-analysis`, `tcl-values` | `tcl_expr_eval` 10, `var_escape` 5, `interprocedural` 3, `type_infer`, `intervals`, `analyses::{ConstValue, LatticeValue}` |
| `tcl-source` | `word_subst` 18, `segmenter` 3 |
| **`tcl-binding`** | `command_binding` 86, `registry_invocation` 67, `environment_ingress` 15, `site_claims` 5, `signature_scan` 2, `var_refs` 2, `const_subst` 2 |
| **`tcl-model` / `tcl-diagnostics`** | `analyser` 9 (`ResolvedAnalysisInput` in production; `Analyser::new` in tests) |
| **`tcl-optimiser`** | `semantic_optimisation` 5 (re-exported at `codegen/wasm/mod.rs:48`) |
| **the unit crate** | `compilation_unit` 18, `semantic_analysis` 1 |

The binding calls are functions, not only types:

- `scan_module_command_mutations_with_bindings`;
- `admitted_native_compiler_invocation` (13);
- `compiled_namespace_context`;
- `original_native_compiler_words`;
- `SourceInvocationBinding::proved_execution_target`.

The unit-crate calls build whole units:

- `CompilationUnit::build_for` (`codegen/emitter/try_blocks.rs:227`);
- `build_with_context_registry` (`inlining/frame.rs:915`, `:1026`, `:1056`);
- `build_with_analysis_input` (`inlining/reference.rs:325`);
- `UnitBuildOptions` (`codegen/wasm/pipeline.rs:865`, `:1117`).

The reason is that `tcl-codegen` holds two things:

1. **Emitters:** IR and native plans to bytecode or WASM.
2. **Drivers:** Tcl source to a module. They run the whole pipeline (unit
   build, semantic optimisation, inlining with callee units).

The drivers are:

- `compile_service` (3.8k lines; `BytecodeCompileService`);
- the WASM `pipeline` (in `codegen/wasm`, 12.6k lines in all);
- `inlining` (8.6k lines).

Its consumers use the drivers almost exclusively:

| Consumer | Uses |
|---|---|
| `tcl-vm` tests (57), `tcl-engine-tclvm` (5), `tcl-irule-test` (4), `tcl-debugger`, `tcl-vm-cli`, `tcl-lsp-db` tests | `compile_service` |
| `tcl-cli` (3), `tcl-mcp` (4) | `codegen::wasm::{compile_wasm, WasmCompileOptions, WasmSemanticDecline, WasmModule}` |
| `tcl-explorer` (14 + 1), `tcl-fuzz` (3 + 2) | emitters (`codegen_module`, `format::format_module_asm_named`, `ModuleAsm`, `Instruction`) and `compile_service` |

So no extraction alone makes `tcl-codegen` depend on only the four crates:
the drivers need the unit crate by definition.

## Design

### Two crates

| Crate | Holds | Depends on | Consumers |
|---|---|---|---|
| `tcl-codegen` (emitters) | `codegen` minus the WASM `pipeline` driver: bytecode and WASM emission, peephole, operand widths, format | `tcl-ir`, `tcl-native-plan`, `tcl-lowering`, `tcl-analysis`, `tcl-values`, `tcl-model`, `tcl-bytecode`, the registry core | `tcl-compile`, `tcl-explorer`'s bytecode view (item 294), `tcl-fuzz`'s emitter differential |
| `tcl-compile` (new; drivers) | `compile_service`, the WASM `pipeline` and `compile_wasm`, `inlining` | the unit crate, `tcl-optimiser`, `tcl-codegen` | `tcl-engine-tclvm`, `tcl-vm-cli`, `tcl-debugger`, `tcl-irule-test`, `tcl-cli`, `tcl-mcp`, `tcl-fuzz`, and the test crates `tcl-vm-it` and `tcl-lsp-db-it` |

### Binding facts reach the emitters as IR

The emitters' binding calls answer per-site questions: is this invocation
admitted to native compilation, what is its namespace context, what is its
proved target? Item 50 carries those answers on the annotated command in
`tcl-ir`. `SourceInvocationBinding` evolves into the annotated command, and
items 248 and 252 decide its home. Where an answer is a plan decision, item 256
puts it in `tcl-native-plan`'s inputs instead. So the 160 `command_binding`
and `registry_invocation` references become reads of `tcl-ir` and
`tcl-native-plan` data. Any call still made by the emitters after step 11 moves
to `tcl-compile`, which may call binding through the unit crate.

`ResolvedAnalysisInput` is an analysis input, so it moves to `tcl-model`, as
the overview's table already says. `try_blocks.rs:227`'s unit build is inside
a test block and moves with the tests.

### The `test-support` dev-dependency on `tcl-compiler`

Today `tcl-codegen`'s tests enable `tcl-compiler/test-support`. It gates 9
items: `SourceAnalysisEntry` constructors, a `unit_queries` accessor, `ir`
span fixtures, `execution_region`, `procedure_identity`,
`native_integer_proof`, `interprocedural`'s `test_body_identity` and
`var_refs::variable_name_role_words`. That costs a second build of the 564k
production lines of `tcl-compiler`, and of `tcl-codegen`, in any worktree that
tests codegen and also builds something else that uses the compiler. The
build-test-cost record shows three `tcl-compiler` incremental directories in
one target.

After the split, each fixture belongs to the crate of the item it builds:

- `tcl-ir`: span fixtures and procedure identity;
- `tcl-native-plan`: execution region and integer proof;
- `tcl-analysis`: body identity;
- `tcl-binding`: `SourceAnalysisEntry` and `var_refs`;
- the unit crate: `unit_queries`.

**Decision: these fixtures are `#[doc(hidden)] pub` with no feature,** following
item 296's recommendation. Two reasons:

- Each is a constructor or accessor of a few lines.
- With the feature, every crate gets a second variant of itself and everything
  above it.

Item 283's "no `#[doc(hidden)]` outside `test-support` items" gate then reads a
`// test-support` marker comment, not the feature, and `retired_api_gate` checks
that only `tests/`, `-it` crates and `src/tests` name such items. If the owner
keeps the feature instead, item 259's script records the duplicate's cost and
item 296 accepts it in its design.

### Decisions

- **Split drivers from emitters** rather than shrink codegen's dependency list
  by moving code into lower crates. A driver depends on the whole pipeline; that
  is what it is for.
- **The new crate is named `tcl-compile`.** It names what its consumers ask
  for: compile this script.
- **Remove the façades** `codegen::helpers` (`codegen/mod.rs:42–43`, a
  re-export of `tcl_compiler::codegen_helpers`) and the
  `semantic_optimisation` re-export (`codegen/wasm/mod.rs:48`). Consumers name
  `tcl-native-plan` and `tcl-optimiser` directly (item 90's no-façade rule).
- **`tcl-vm` stays a dev-dependency of the emitter crate** for tests that run
  emitted bytecode. `tcl-vm` depends on neither crate, so this causes no
  cascade. Item 291 shrinks it.

### Open questions

- Is any part of `inlining` an IR-to-IR pass that needs no unit build? If so, it
  could live in the emitter crate. The question is what `inlining/frame.rs`
  needs callee units for: if only callee IR, a `CalleeBodies` input built by
  `tcl-compile` replaces the three unit builds. The inventory under item 283
  answers it.
- Which of the 160 binding references are on the annotated command after item
  50? Recount when item 252 lands, with
  `rg 'tcl_binding::' rust/tcl-codegen/src`.

## Work list

1. After item 252: re-point `ir`, `cfg` and `expr_ast` to `tcl-ir`.
2. After items 254–256: re-point lowering, analysis and native-plan paths.
   Replace `codegen::helpers` users with `tcl_native_plan::codegen_helpers`.
3. With item 50 and item 283: move emitter binding calls to annotated-command
   reads or native-plan inputs. Record each remaining one.
4. Create `rust/tcl-compile`. Move `compile_service`, `codegen/wasm/pipeline.rs`
   with `compile_wasm` and its options, and `inlining`. Rewrite the consumers in
   the same change; there is no re-export left behind.
5. Move the per-crate fixtures and drop `features = ["test-support"]`.
6. Re-key the shard map, heavy tests, `wasm-real-link`'s `-p` and the path filters
   that name `rust/tcl-codegen` (build-test-cost item 4 lists them) for the new
   crate.

## Verification

Done when (item 292's own, tightened):

- `cargo tree -p tcl-codegen -e normal` lists no `tcl-compiler` (unit crate),
  `tcl-binding`, `tcl-diagnostics` or `tcl-optimiser`;
- an analyser edit (item 259's `analyser` edit) compiles no `tcl-codegen`
  unit; it does compile `tcl-compile`, and the record says so;
- `cargo tree -p tcl-codegen -e dev --depth 1` lists no `test-support` feature
  on any split crate;
- the moved tests give the same outcomes by name (`failset.py compare`).

Measurements: the units item 259's script rebuilds after the analyser and
lexer edits, before and after, and the WASM output of `wasm-real-link`
unchanged byte for byte on its fixtures.

## Risks

- **Two crates where consumers expect one.** Both are named in each consumer's
  manifest. `tcl-explorer` needs both, which item 294 plans for.
- **Generic code instantiated downstream** (the overview's risk) grows with one
  more boundary. Watch release binary size and the WASM builds.
- **Tests that span both.** Emitter tests that build units today (the
  `structured.rs` and `try_blocks.rs` test blocks) move to `tcl-compile`'s tests
  or use `tcl-lowering` fixtures directly.

## Corrections to the overview

- Item 292's list (`tcl-native-plan`, `tcl-lowering`, `tcl-analysis`, `tcl-ir`)
  holds only for the emitters. The drivers need the unit crate and
  `tcl-optimiser`, and the emitters need binding until item 50's annotated
  commands carry its answers. Item 292 should name the driver crate.
- The brief for this design listed `tcl-native-plan`, `tcl-lowering` and
  `tcl-ir` only. The code also needs `tcl-analysis` and `tcl-values` (value
  facts and escape), and `tcl-model` (`ResolvedAnalysisInput`).

## Owner decision (26 November 2026)

The split is adopted. `tcl-codegen` keeps the emitters, and the new `tcl-compile` crate holds the drivers above the unit crate. Step 11 follows this design.
