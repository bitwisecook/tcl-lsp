# Step 8: extract tcl-binding, and place tcl-invocation

Part of [splitting tcl-compiler](../crate-split-design.md). Plan items 253
(extract `tcl-binding` as it is) and 282 (place `tcl-invocation`), in that
order. Builds on [step 7](07-binding-fixpoint.md), which inverts binding's
edges into lowering, analysis and the analyser. Steps 9
([`tcl-lowering`](09-tcl-lowering.md)) and 10
([`tcl-analysis`](10-tcl-analysis.md)) build on it.

**Decision for item 282:** `registry_invocation` stays inside `tcl-binding`.
There is no `tcl-invocation` crate in this split. The question reopens after
item 84.

## Problem

Binding is the largest part of `tcl-compiler`: 223k production lines
(measured as in step 7: test-named files and `#[cfg(test)]` items removed,
26 November 2026).

| Module | Production lines |
|---|---|
| `command_binding` | 151,978 |
| `registry_invocation` | 39,472 |
| `signature_scan` | 11,176 |
| `var_resolve` | 8,466 |
| `variable_bindings` | 4,810 |
| `realm` | 3,908 |
| `ir_helpers` | 3,214 |
| `dynamic_names` | 1,893 |
| `raw_binding` | 1,041 |

Inside the group, references run in every direction (2,785 in all). The
largest pairs:

| Pair | Refs (files) |
|---|---|
| `command_binding` → `registry_invocation` | 781 (162) |
| `command_binding` → `var_resolve` | 458 (104) |
| `command_binding` → `signature_scan` | 317 (110) |
| `registry_invocation` → `command_binding` | 268 (52) |
| `realm` → `command_binding` | 136 (7) |
| `registry_invocation` → `signature_scan` | 114 (26) |
| `command_binding` → `ir_helpers` | 103 (20) |

Outside the group, binding names `ir` 929 times, `segmenter` 216, `place`
200, `native_numeric` 50, `tcl_expr_eval` 37, `ssa` 31, `word_subst` 28 and
`cfg` 22. A tail of 26 small modules has 1–16 references each.

## Design

### What moves in

- **The overview's list:** `command_binding`, `registry_invocation`, `realm`,
  `raw_binding`, `variable_bindings`, `var_resolve`, `signature_scan` and
  `dynamic_names`.
- **`ir_helpers`.** Binding names it 103 times. It names `command_binding` 7
  times and `registry_invocation` 14 times, and every other user sits above
  binding (`cfg_builder` 57, `interprocedural` 12, `optimiser` 8, `sccp` 5,
  `ssa` 5).
- **`ssa::uses`** and its `word_scan` and `script_reads` submodules (1.5k
  lines). These are syntactic def/use scans of `Statement`s (step 7).
- **The small modules that name binding back.** They cannot sit below it, and
  their only edges up are the few noted, which steps 4, 5 and 7 clear:

| Module | Lines | binding → it / it → binding |
|---|---|---|
| `native_numeric` | 1,309 | 50 / 33 (and 14 into `tcl_expr_eval`, which step 4 clears) |
| `place_bridge` | 1,732 | 10 / 54 (plus 1 each into `ssa` and `memory_ssa`) |
| `math_function_binding` | 1,002 | 4 / 36 (plus 7 into `tcl_expr_eval` and 1 into `optimiser`) |
| `existence_query` | 669 | 2 / 31 |
| `allocated_instance` | 482 | 10 / 26 |
| `array_destruction` | 421 | 4 / 13 |
| `lambda_literal` | 411 | 1 / 10 (plus 3 into `analyser`, which step 5 clears) |
| `literal_contents` | 279 | 2 / 10 |
| `dictionary_bindings` | 769 | 5 / 7 |
| `script_binds`, `captured_cell`, `var_refs` | 332, 311, 830 | 14 / 6, 13 / 4, 13 / 3 |

- **What goes elsewhere.** The one-way helpers go down: `environment_ingress`,
  `analysis_control`, `alias`, `declared_frame_effects` and `contents_source`
  to `tcl-model`, and `depth_guard` to `tcl-source` (step 2). `object_types` (4,375
  lines) names the analyser 22 times and `compilation_unit` 7 times, so it
  sits above binding. Binding's 4 references to it
  (`registry_invocation/source_structure/project_class_cards.rs`) move up with
  that query module, or the candidate records move to `tcl-model`.
- **`unit_scope` does not move in.** The overview lists it, but it lowers
  source and builds CFGs (`unit_scope/source_scan.rs:234`, `extra_scan_contexts.rs`).
  Its only users are `compilation_unit` (20) and `analyser` (2), so it goes to
  `tcl-unit` ([step 13](13-unit-and-db.md)).

### Item 253: a pure move

The modules move unchanged into `rust/tcl-binding/src/`, with the same module
tree. Folding `signature_scan` into the one binding walk stays item 84's
work, and so does the `registry_invocation` reshaping below.

```toml
[dependencies]
tcl-source = …; tcl-names = …; tcl-values = …; tcl-model = …; tcl-ir = …
tcl-registry = …; tcl-dialect = …; tcl-lexer = …; tcl-syntax = …; tcl-core-types = …
```

No dev-dependency sits above `tcl-binding`. The 12 SpecTcl-using test blocks
left in step 0 (`jim_stock_class.rs`, `jim_wrapper_body.rs` and others) are
already in `tcl-compiler-it`.

**Public interface.** Today `tcl_compiler::command_binding` and
`registry_invocation` are `pub mod` with 102 `#[doc(hidden)]` items (62 and
40). The crate exposes:

- `tcl_binding::api`: `SourceCommandBindings`, `ModuleCommandBindings`,
  `CommandBindingRealm`, `SourceInvocationBinding` (if step 6 left it here),
  `SourceAnalysisOptions`, `SourceAnalysisEntry`, the `invocation_at_source`
  query, `report::BodyReport` and `fixpoint::ProvedOperands`;
- `tcl_binding::native`: the compiler-facing facts `tcl-codegen` reads today
  through hidden items ([step 11](11-tcl-native-plan.md) names them);
- `tcl_binding::query`: the source-structure queries that LSP features call
  (today `registry_invocation::source_structure`, 11.1k lines).

Everything else is `pub(crate)`. Where a `pub(crate)` item is needed across
the new boundary, the boundary picks the `api` path. No module is re-exported
wholesale.

**Consumers rewritten in the same change.** These are the files that name
`tcl_compiler::command_binding` or `registry_invocation` today:

| Crate | `command_binding` / `registry_invocation` files |
|---|---|
| `tcl-lsp-core` | 76 / 114 |
| `tcl-codegen` | 42 / 27 |
| `tcl-lsp-db` | 13 / 15 |
| `tcl-lsp-server` | 11 / 13 |
| `tcl-irules`, `tcl-spectcl`, `tcl-diagram`, `f5-xc`, `tcl-explorer`, `tcl-cli`, others | 1–5 each |

Inside `tcl-compiler`, every `crate::command_binding::` path becomes
`tcl_binding::`. `retired_api_gate` then forbids the old paths.

### Item 282: where tcl-invocation goes

The question was whether `registry_invocation`, the registry adapter, can
become a crate below `tcl-binding`. The ledger says no:

- **It needs binding's results.** `registry_invocation` → `command_binding` is
  268 references to 106 distinct items. They include `SourceCommandBindings`
  (11), `OriginalCompilationLookupAdvice` (10),
  `OriginalSourceCommandTransitionAdvice` (9), `SourceInvocationBinding` (7),
  `CommandAllocationSite` (9) and the conditional-source machinery
  (`OriginalSourceConditionalRegion`, `SourceConditionalBodyEntry`,
  `OriginalSingleSourceCutSurvey`).
- **Little of it is a pure adapter.** Only 9 of its 77 production files
  (2,229 of 39,472 lines) name no binding, analyser or lowering module.
- **Its largest part is a query layer.** `source_structure` (11.1k lines) has
  244 references to binding and the analyser. It answers LSP questions over a
  finished analysis (`original_logical_source_words_at(source, analysis,
  offset)` and the like).

Splitting it below binding would mean moving about 100 binding types down
with it, which is most of binding's vocabulary. The decision is to keep it
inside `tcl-binding`.

- **`source_structure`** is already its own module tree. After item 250 it
  reads reports only through `BodyReport`. It is the first candidate for item
  85's `tcl-semantic` query crate, above binding and analysis.
- **The adapter core** (`invocation_word`, `resolved_handler_invocation*`,
  `InvocationMetadataContext`, `effective_invocation_word`, and the 9 clean
  files) is measured again after item 84. If it then has no back-edge to
  `command_binding`, `tcl-invocation` becomes a crate below `tcl-binding`.
  Item 84's single binding walk is what decides the `realm` and transition
  advice types it reads today.

The overview's crate table records `tcl-invocation` as "not split; see step
8".

## Work list

1. Check the ledger: `module-back-edges.tsv` rows from every module listed
   above into lowering, SSA, analysis and the analyser read 0 (step 7 done),
   and into `tcl-source`, `tcl-names`, `tcl-values`, `tcl-model` and `tcl-ir`
   only.
2. Move the 9 overview modules, `ir_helpers`, `ssa::uses`, and the 12 small
   modules in the table into `rust/tcl-binding`. Move the 6 one-way helpers
   to their lower crates.
3. Write `tcl_binding::{api, native, query}`. Turn the hidden items into
   `pub` items of those modules or into `pub(crate)`.
4. Rewrite consumer paths in `tcl-compiler` and the crates above. Add
   `tcl-binding` to their `Cargo.toml`, and remove `tcl-compiler` where it
   was only reached for binding.
5. Gates:
   - add `deny.toml` `wrappers` for `tcl-binding`;
   - re-root `walker-ownership`, `owner-resolution`, `registry-axes` and
     `segmentation-drift`;
   - point the shared-utility contracts' owner-crate column at `tcl-binding`
     where it says `command_binding`.
6. Record the crate table entries for `tcl-binding` and `tcl-invocation`.

## Verification

- **Item 253:** `rust/tcl-binding` builds, its back-edge rows read 0, the gates
  name its paths, and the failset comparison against the last green commit
  before the move shows no change.
- **Item 282:** the crate table says `tcl-invocation` is not split, and this
  doc gives the reason.
- **Measured** (item 90's record):
  - `cargo tree -p tcl-binding -e normal` crate count. Expected: about
    the registry's 54 plus the five vocabulary crates.
  - Crates rebuilt after an edit to `command_binding/` with `cargo test -p
    tcl-binding --lib --no-run`. Expected: 1.
  - Crates rebuilt after an edit to `analyser/` or `sccp/`. Expected: no
    binding crate.
  - Peak rustc RSS for `tcl-binding`, against today's `tcl-compiler`.
  - Release analysis latency on the analysis-performance corpus, cold and
    after a one-line edit, and the VM benchmarks (`scripts/perf/interp/`),
    before and after.

## Risks

- **It is still a 226k-line crate.** It is the largest compile unit left.
  Peak memory and incremental time improve less than for the other crates.
  The next cut is item 84's single walk, not a module split now.
- **Inlining across the boundary.** Hot binding queries called from SSA and
  SCCP (`find_at_source`, `invocation_at_source`) lose inlining in
  opt-level-1 test builds. Release builds use LTO. Watch the latency record.
- **Consumer churn.** About 300 files outside `tcl-compiler` change paths in
  one commit, and work in flight on them conflicts. Agents finish their
  current item at a clean point first (the overview's "work in flight").

## Corrections to the overview

- `unit_scope` goes to `tcl-unit`, not `tcl-binding`: it lowers and builds
  CFGs, and only the unit and the analyser use it.
- `ir_helpers` goes to `tcl-binding`, not `tcl-lowering`.
- Twelve small modules (`native_numeric`, `place_bridge`,
  `math_function_binding`, `existence_query`, `allocated_instance`,
  `array_destruction`, `lambda_literal`, `literal_contents`,
  `dictionary_bindings`, `script_binds`, `captured_cell`, `var_refs`) and the
  `ssa::uses` scans join `tcl-binding`.
- `tcl-invocation` is not extracted (item 282's "or the design records why"
  branch). The overview's sentence "placed below binding if the ledger allows
  it" resolves to "it does not".
- Steps 2 and 6 inherit edges into binding that their docs must clear:
  - `word_subst` (`tcl-source`) names binding 42 times;
  - `cfg` (`tcl-ir`) names binding 21 times;
  - `segmenter` names the analyser 5 times.
