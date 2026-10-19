# Merge-B notes: origin/rust (fcb7927f0) into work (18ca4d3ca), set-B

Merge base 127dba9d9. Merge-tree output tree a4e62fd92.

Drafts are in `resolved/<path>`: all 60 set-B files, plus three extra files: `tcl-dialect` `grammar.rs`, `codegen/native_error.rs` and `tcl_expr_eval/native_constant.rs` (see below). None were compiled, as instructed.

Method:
- Hunks were taken from `git merge-file --diff3` (scripts in `b3/res/*.py`, applied by `b3/apply.py`).
- Where diff3 alignment interleaved unrelated code, the file was rebuilt as "ours plus upstream's semantic changes" (`b3/xf/*.py`). This applies to:
  - `cmd_subst.rs`
  - `values.rs`
  - `command_binding.rs`
  - `tcl_expr_eval.rs`

## Per-file resolution

### Docs and scripts
- `docs/design/compiler/semantic-aot-optimisation.md`: ours text, plus upstream's regsub/compiled-regexp sentence.
- `docs/design/compiler/wasm-target-surfaces.md`: ours, plus upstream's `encoding convertfrom/convertto` paragraph.
- `docs/design/contracts/development-environment.md`: both sides (ours, then theirs).
- `docs/design/contracts/shared-utility-contracts-rust.md`: ours rows, plus theirs' new rows (utf.rs, frame, path names, encoding convertfrom) and symbols (`dict::key_not_known`, `print_double`, `validate_operators`/`operator_admitted`, `dispatch_under`, `resolve_profile`).
- `docs/generated/wasm-command-backing.md`: ours, plus upstream's `representation` rows (release 9.0). Counts recomputed: builtin 303, total 394. This is a generated file; regenerate after compile.
- `docs/references/command-spec/fields.md`: both hook tables merged.
  - Codegen: `Uplevel`, `Set`, `ArraySet`, `ArrayUnset`, `Variable`, `NamespaceUpvar`.
  - Inline: `Yield`, `YieldTo`, `Set`, `StringIs`…`StringTrimRight`.
  - The duplicate `Set` row was removed.
- `scripts/dev/rust-test-binary-shards.tsv`: union with no duplicate rows; upstream weights kept.

### tcl-cli, tcl-dialect, tcl-explorer, tcl-lsp
- `rust/tcl-cli/src/commands/pkg.rs`: theirs `build_profile` (`as_os_str`), on top of ours.
- `rust/tcl-dialect/src/lib.rs`: export union. Exports:
  - `DoubleSpelling`, `NumberSpelling`, `SourceLineEndings`, `CharacterUnits`, `PackageDiagnostic`, `UnitRun`, `tcl84_formal_integer_end`.
  - Theirs' `grammar::DoubleFormat` is not re-exported. Ours `DoubleFormat` stays canonical.
- `rust/tcl-registry/src/commands/tcl/string_.rs` (note): `string_is_member_type_for_dialect` now calls the 2-argument `resolve_class(word, release)` (`point.tcl_version()`, else 9.1), so the class word abbreviates against the release's own table.
- `rust/tcl-dialect/src/version.rs`: theirs `units()`/`CharacterUnits`/`UnitRun` with `Jim084Utf8` arms; ours `validate_requirement_for`; theirs test (uses `reported`).
- `rust/tcl-explorer/src/serialise.rs`: ours. The bounds view passes a `BoundsRelease` with `lset_appends_at_length_for(profile.const_fold_version())` to `find_interval_bounds_in_release`, as upstream's serialise does.
- `rust/tcl-lsp-db/src/lib.rs`: ours (superset).
- `rust/tcl-lsp-server/src/lib.rs`: ours. It is a superset: `mark_started` plus the inventory-scan invalidation.

### tcl-lexer
- `expr_lexer.rs`: ours. Upstream's lone `$` on 8.4 is ported: `Ok(None)` rejects only when `!lone_dollar_is_text(expr_grammar_base)`.
- `ranges.rs`: theirs word-by-word command-substitution scanner. A `braced_var: BracedVarStyle` parameter is threaded through `scan_command_substitution`/`word_char_end`/`variable_name_end`. `command_substitution_end_bytes_in(bytes, open, config)` delegates to it.
- `lexer.rs`: theirs scanner call. Ours JimNative path is kept, plus `Continuation::Document` when `source_map.channel()==SourceChannel::Document`. `advance_command_comment` was removed.
- `structural_index.rs`: theirs `${` scan via `scan_dollar_brace(..., config.braced_var)`.
- `word_parts.rs`: `close_quote_offset_bytes` returns `ComponentParseError` and walks every `$` with `scan_var_ref_with_term`.

### tcl-registry
- `commands/tcl/lseq.rs`: ours.
- `commands/tcl/set_.rs`: import union (`CommandForm`, `CodegenHookId`, `InlineCodegenHookId`, `LoweringHookId`).
- `commands/tcl/string_.rs`: both sides.
- `expr_surface.rs`: `validate` made generic. Theirs `validate_operators<Text>`/`operator_admitted` exposed.
- `hooks.rs`: discriminants merged.
  - Codegen: `Upvar` 14, `Uplevel` 15, `Set` 16, `ArraySet` 17, `ArrayUnset` 18, `Variable` 19, `NamespaceUpvar` 20.
  - Inline: `Set` 25, `StringIs` 26, `StringMatch` 27, `StringReplace` 28, `StringTrim` 29, `StringTrimLeft` 30, `StringTrimRight` 31.
- `value_transfer/completion.rs`: ours.
- `value_transfer/const_ops.rs`: theirs `NumberSpelling`/`spell_*` and `precision_written` in `TargetSemantics`. The spelling is filtered by `grammar.numbers`.

### tcl-spectcl
- `catalogue.rs`: union of hook ids.
- `loader.rs`: both sides, plus `CodegenHookId::Uplevel` in the loader table.

### tcl-syntax
- `expr/errors.rs`: theirs docs. The bytes path decodes via `c_string_extent`, then `desc.of_value`/`for_release`.
- `expr/mathfunc.rs`, `expr/syntax_error.rs`: both sides.
- `number.rs`: both sides. `format_double_with` maps onto ours `DoubleFormat::{Shortest, TclSignificant, General}`.
- `value.rs`: theirs `NotIntegerBytes`/`DoubleNotInteger` plus ours byte variants. `error_code` has bytes arms and `_ => None`. `as_chars` delegates to ours `try_as_str`. `measure_value` uses `as_bytes`.

### tcl-compiler: analyser, IR, optimiser, tests
- `analyser/bounds_checks.rs`: theirs new tests plus ours docs.
- `analyser/commands.rs`: `file_lexer_config()` plus theirs `BoundsRelease{numbers, ..self.bounds_release()}`.
- `analyser/diagnostics/dataflow.rs`: ours. The interval-bounds diagnostic now reads `self.bounds_release()` (characters, numbers and the `lset` append fact) through `find_interval_bounds_in_release`, as upstream does.
- `ir.rs`: ours fields. Added `top_level_evaluation: CommandEvaluation` on Module (auto-merged) and `pub shape: InvocationShape` on `CommandBindingSite`.
- `interval_bounds.rs`: ours plus upstream's `lset` release fact. New `find_interval_bounds_in_release(…, release: BoundsRelease, semantics)`; `find_interval_bounds_resolved` delegates with `lset_appends_at_length: None`. `BoundsCtx` carries `lset_appends_at_length`. `classify` reports `past_append` at `index == length` when the selected invocation is `ListSetBounds::ExistingElement` (ours) or the release refuses the append (`Some(false)`, upstream). `find_interval_bounds_with` passes its whole `BoundsRelease`.
- `optimiser/branch_folding.rs`, `tests/codegen_integration.rs`: ours. Ours has no `Module` struct literal there for upstream's `top_level_evaluation` line.
- `optimiser/propagation.rs`: ours, plus upstream's O129 filter. Ours' O129 core is `fold_retained_builtin`, not upstream's deleted `fold_builtin_cmd_subst_raw`. It gains a `dialect` parameter (`ctx.dialect` at the 4 production sites, `None` in tests) and drops a result that `folded_literal_reads_apart` under `dialect.vm_runtime_version`. Upstream's O116 hazard assertions (`o116("tcl8.6") == 0`, `o116("tcl9.0") == 1`) are present (auto-merged).
- `inlining/mod.rs`: both sides.
- `const_subst.rs`: theirs `invocations: Vec<FoldedInvocation>` (binding plus `InvocationShape`). It is built with ours `.with_namespace_context(...)`. Also `folded_literal_reads_apart`.

### tcl-compiler: CFG
- `cfg.rs`: both sides (`condition_binding_sites` plus `condition_owners`).
- `cfg_builder/mod.rs`: theirs fields. Ours `CommandBindingSite` construction now includes `shape`. Also `record_body_error_sites(stmt)` before ours evaluated-body region, and the For arm uses `raw_tokens.as_deref()`.
- `cfg_builder/cfg_lower.rs`: theirs `condition_owners` inserts on ours `set_terminator`. The catch owner is threaded through `lower_body(owner.as_ref(), …)`.

### tcl-compiler: codegen
- `codegen/mod.rs`: theirs direct-evaluation/release-binding fields and methods were ported. This includes:
  - `expr_depth`, `direct_commands`, `in_direct_command`, `release_binding_of`, `release_invoke_binding`, `invocation_shape`, `hold_release_binding*`, `stamp_substitution_error_info`, `error_info_source_text` and `direct_statement_error_info`.
  - These run on ours `SourceImage`/`try_text`, `require_command_site`, `native_site_binding_requirement` and `invocation_surface_query`.
- `codegen/cmd_subst.rs`: whole port onto ours. Ported pieces:
  - `try_emit_compiled_index_command` and `parse_time_expansion`
  - `release_compiled` holds and substitution errorInfo stamping
  - `names_builtin` (`::cmd` compiles like `cmd`)
  - The `re_to_glob` compiled-regexp path. Ours `retain_entered_command` and native operand words are kept.
  - `info exists arr(key)` emits `EXIST_ARRAY <local>` with the key pushed (`emit_original_array_exists_key`). It takes the original operand layout, or a hand-built context's text. Upstream's `TclCompileInfoExistsCmd` shape.
  - `emit_generic_cmd_subst` interns the entered command's compiled locals.
  - `emit_inline_incr` passes the amount's bracedness.
  - Upstream's 3 new tests:
    - `a_literal_insert_index_past_int_is_respelt_where_the_release_compiles_it`
    - `a_literal_end_offset_past_int_is_fixed_where_the_release_compiles_it`
    - `qualified_head_word_compiles_as_the_builtin`, which replaces ours' contradicting `qualified_head_word_stays_generic`, as upstream did.
  - Upstream's edits to `emit_braced_arg_as_exact_verbatim_literal`, `inline_lrange_end_plus_n_falls_back_to_non_imm` and `registry_inline_hook_stamping_matches_previous_hardcoded_dispatch`. The last adds `Set`, `string match` and the 8.6 `string` subcommand hook windows.
- `codegen/control_flow.rs`: theirs `release_compiled`/`CompileWord`, compiled-index catch arm and decline-branch local interning, on ours `invocation_surface_query`. Upstream's new `lower_inline_body` (body rebased to its source offset) is covered by ours' `authored_inline_body_base`/`with_inline_body_operand`, which place an inline body at its original source coordinates. Upstream's decline-path interning for `dict for`/`dict update`/`dict with` is present. For `dict map` it is ported into `emit_dict_map`'s decline branch.
- `codegen/emitter/bytecoded.rs`: hook-table union, plus `codegen_hook_locals`. That is upstream's `applicable_codegen_binding` probe returning the probe's local table, recast for ours' surrogate. Ported:
  - `is_compilable_local` gate
  - `script_is_literal` for `dict update`/`dict with`
  - the default-shape `require_site_binding`
- `codegen/emitter/generate.rs`: theirs local interning before ours targets, plus:
  - `ctx.require_command_site(site)` and `laid_out` from `block_order`
  - `BodyTree::new(cfg)` and arm `pattern.unwrap_or_default()`
  - dead-locals via `try_text`
- `codegen/emitter/mod.rs`: `direct_commands` copied into the configured codegen context (non-proc) and into the `ModuleEmit` literal.
- `codegen/expressions.rs`: theirs `expr_depth` wrapper (`emit_prepared_expr_at_depth`) and `is_foldable_operation`/`compiler_pools_constants`/`fold_integer_expression`/`spell_wide` over ours `FoldPolicy`.
- `codegen/helpers.rs`: ours, minus `regexp_to_glob`. It moved to `tcl_syntax::glob::re_to_glob`, as upstream did.
- `codegen/statements.rs`:
  - `AssignExpr` uses ours `emit_assign_expression`, which now stamps the substitution errorInfo from `fallback_value`.
  - `Incr` uses ours, passing upstream's `*amount_braced` to the 4-argument `emit_incr`.
  - The whole-cmd-subst path uses ours `with_invocation_tokens`, plus theirs errorInfo stamp.
  - `emit_word_arg` uses ours.
  - The generic invoke uses ours (entered surrogate and original head) plus theirs `release_compiled`/`release_binding` on the INVOKE. When the entered surrogate is selected, it interns the hook's compiled locals (`intern_entered_command_locals`).
- `codegen/values.rs`: theirs filter on folded literals (`folded_literal_reads_apart`), invocation-shaped binding requirements, List `literal_rep`, and `intern_compiled_local`/`intern_script_locals`, on ours. Also:
  - Every `push_verbatim = true` site also sets `literal_rep = LiteralRep::Verbatim`.
  - `emit_incr(name, key_is_literal, amount, amount_braced)` is 4-argument.
  - New `intern_entered_command_locals`.
  - Upstream's new test `a_folded_list_pushes_a_list_and_a_word_read_apart_stays_unfolded`, plus upstream's `literal_rep` assertions in two existing tests.

### tcl-compiler: other
- `command_binding.rs`: ours plus theirs `precision_written`:
  - the field and docs, and `PrecisionWrites`
  - `module_may_set_precision`, which uses a byte-window search of the source
  - distrust/snapshot/projection handling
  - tests updated to 8 fields
- `compile_service.rs`:
  - Theirs `compile_target_with(target: ScriptCompileTarget, …)`, which sets `ir.top_level_evaluation`, on ours `first_parse_cut_image_checked` and `lower_script_module_for_bytecode_with_options`.
  - Ours `compile_script_with_entry` passes `target`.
  - Every `ScriptCompileTarget` literal in this file gains `evaluation: CommandEvaluation::Compiled`.
  - Theirs START_CMD test was added. Ours renamed test was kept.
- `lowering/mod.rs`:
  - Ours `CommandBindingSite` constructors gain `shape: self.invocation_shape(seg)`.
  - The Catch arm declines when `release_admits_arity(requirement.binding_identity(), args.len())` fails.
  - `eval $var` is not folded for bytecode targets.
  - An `eval` list whose word reads apart is not folded (`folded_literal_reads_apart`).
  - Ours `SourceSegmentationPurpose`/`original_native_written_argument_value` are kept.
- `lowering/structured.rs`: theirs `condition_expr` (simple word, otherwise `ExprNode::substituted_word`) with ours `parse_expr_for_profile_with_lexer_config(…, self.config)` and ours `script_word_span`.
- `tcl_expr_eval.rs`: ours plus theirs:
  - `FoldPolicy::{numbers_or_inferred, spelling}`
  - `operators_in_release` (fold, and services return `WrongRepresentation`)
  - out-of-range double literal is past the 8.4 tower
  - `i64::MIN % -1` trap decline
  - untrimmed boolean read
  - `NumberSpelling` field and `text_of` (INT64_MIN garbling) on ours `to_string_val(format)`
  - services `precision_written`
  - `full_value` via `spell_wide`/`spell_double` (`Unspellable`)
  - tests; the iRules `in`/`ni` tests now expect `None`
  - upstream's `declined: Option<DeclineReason>` refactor. Replaces ours' `ambiguous`/`platform` flags; the first reason stands. The 2^63 comparison now declines `ReleaseAmbiguous(Platform)`, as upstream does (ours had folded it into `NumeralGrammar`). `text_of` records `Unspellable` for an INT64_MIN the release garbles, or for a double whose precision may be written. Services' string ops report the recorded reason (`fold_refused`).
  - `dispatch_under` gate in both math dispatchers. The Tcl table declines Jim's integer `pow`; the Jim protocol computes it natively. New test `jims_integer_pow_never_folds_to_tcls_double`.

### Extra files outside set-B
- `rust/tcl-dialect/src/grammar.rs`: edited from the auto-merged blob. Theirs `DoubleFormat` enum (`Shortest`/`Significant`/`Printf`) was removed in favour of ours `crate::DoubleFormat`. `double_format(self, precision)` maps:
  - Tcl84 → `General(p)`
  - Jim → `General(12)`
  - Tcl85/90 at precision 0 → `Shortest`
  - otherwise → `TclSignificant(p)`

  Whoever owns the final merge must take this file from `resolved/` too.
- `rust/tcl-compiler/src/codegen/native_error.rs` (auto-merged blob, edited). This is ours' equivalent of upstream's `emit_catch_error`. It takes upstream's `source_command.trim_start()` in place of `trim()`, so a trailing space stays in `errorInfo`. With no explicit text, it uses `error_info_source_text(current_span)`: the command through the whitespace before its terminator, as upstream's `emit_try_body_stmt` does.
- `rust/tcl-compiler/src/tcl_expr_eval/native_constant.rs` (auto-merged blob, edited):
  - `ops.ambiguous` → `ops.declined.is_some()`
  - operand text through `text_of`
  - `illegal_operand_error_code(desc, &bytes, version)` (3-argument)

## Risky / needs compile attention
1. **`grammar.rs` extra edit** (above). Every upstream user of `grammar::DoubleFormat::{Significant, Printf}` outside set-B must be moved to `General`/`TclSignificant`.
2. **`ScriptCompileTarget` gained `evaluation`** (tcl-runtime-api, auto-merged). Ours literals outside set-B need `evaluation: CommandEvaluation::Compiled`:
   - `rust/tcl-compiler/src/command_binding/runtime_entry.rs:2341`
   - `lowering/native_control_carrier_tests.rs:74,207`
   - `rust/tcl-vm/src/interp.rs:17738,25698,25734`
   - `rust/tcl-vm/tests/command_mutation_deopt_e2e.rs` (7 sites)
3. **`CommandBindingSite.shape` is a new required field.** Other constructors outside set-B, such as `execution_regions.rs:~217`, need `shape: Default::default()` or a real shape.
4. **Module literals without `..Default::default()`** need `top_level_evaluation`: `compilation_unit.rs:~3375` and `deferred_writes.rs:~899`. `command_binding.rs` `recovered_procedure_source` already uses `..Module::default()`.
5. **`illegal_operand_error_code` 2-argument callers.** `native_constant.rs` is fixed (extra file). tcl-vm `expr.rs` is Merge-A's. runtime/rust `expr.rs:412` (auto-merged) still has the 2-argument form.
6. **`InlineBodyErrorSite`/`BodyErrorContext` field changes** affect ours code and tests, such as the generate.rs test near line 1532.
7. **Dual literal flags.** The VM reads `push_verbatim`; upstream added `literal_rep`. Both now exist. Codegen sets both together: Verbatim with `push_verbatim`, and List also sets `push_verbatim`. Check the VM gives `LiteralRep::List` its canonical-list reading.
8. **`parse_tcl_index` removed upstream**, replaced by `tcl_cmd_core::index::{encode, Encoded, encodes_apart}`. Remaining ours callers must move.
9. **`as_chars` signature** (tcl-syntax `value.rs`) now returns `Result<Rc<str>, UnicodeAccessError>`. Check its callers.
10. **`set_runtime_print`/`print_double` consumers** (tcl-vm, runtime, tcl-cmd-core `binary.rs`) depend on set-A naming.
11. **The new command-substitution scanner in `ranges.rs` does not apply ours lone-CR translation.** Verify Document-channel CR handling.
12. **`names_builtin`** (`::cmd` compiles like `cmd`) is a name-based path. The tests `catch_body_qualified_error_compiles_as_error` and `qualified_head_word_compiles_as_the_builtin` depend on it.
13. **`require_site_binding`** now takes a shape. Callers outside set-B must pass `&InvocationShape::default()` or a real shape.
14. **`emit_incr` is now 4-argument** (upstream's `amount_braced`). No callers outside set-B exist in the merge tree.
15. **Lone `$` on 8.4 expr** is now a Variable token or text, per `lone_dollar_is_text`. Check the expr parser handles it.
16. **Text gates.** `intern_compiled_local` and `is_compilable_local` are gated on `&str` names; ours LVT is `NameBytes`. Non-UTF-8 local names are not interned.
17. **`release_compiles_control`** reads the release from the registry profile (`vm_release`).
18. **Possible duplicate methods.** `has_package_requirements` and `has_package_requirement_ranges` both exist after the merge. Check for a dead-code warning or duplication.
19. **`ValueError::error_code` has `_ => None`** for list/byte errors.
20. **External symbols assumed from other agents' merges or the auto-merge:**
    - `tcl_cmd_core::index::{encode, Encoded, encodes_apart}`
    - `tcl_cmd_core::string::default_trim_set`
    - `tcl_syntax::glob::re_to_glob`
    - `tcl_registry::release_compilation::*`
    - `tcl_bytecode::{ReleaseCompiled, ErrorInfoCommand, end_relative_imm}`
    - `crate::compile_84`
21. **`statements.rs` `AssignExpr` errorInfo stamp** passes `None` tokens, as upstream does.
22. **`lowering/mod.rs` `eval $var` change.** In bytecode targets it now declines to fold, which is upstream's C fidelity. Ours tests that expect a bytecode-folded `eval $var` may need updating.
23. **`tcl_expr_eval.rs` `full_value` doubles** are now spelled by `NumberSpelling`: `%.12g` on 8.4/Jim, and none when precision is written. Previously ours used shortest `format_double`. Analysis answers for 8.4 doubles may change or decline.
24. **`lowering/structured.rs` substituted conditions.** A non-simple word condition (`if $c`) becomes `ExprNode::substituted_word` (`[expr "…"]` Command node) rather than a parsed expression. Analyses that matched `ExprNode::Var` for these may change.

25. **Upstream's `registry_inline_hook_stamping…` test runs on ours' 9.1-projected registry.** If projection stamps windowed `string` hooks into `inline_codegen_hook`, its "nothing else" loop needs those subcommands added to `expected_subcommands`.
26. **`codegen_hook_locals` probe context.** It copies numbers/escapes/word_rules/braced_var/dialect/arg bracedness/current span/direct commands/invocation tokens. It does not copy `native_compilation`/`native_entry`, so a hook that requires them declines in the probe and interns nothing.
27. **`tcl_expr_eval` decline reasons.** The 2^63-sliver comparison is now `ReleaseAmbiguous(Platform)`. Services' string-op failures over unspellable numbers are now `Unspellable`; they were `WrongRepresentation`. value-transfer tests that pin the old reasons may need updating.

## Naming-system bypasses ported or flagged
- `lowering/mod.rs` `release_admits_arity`: upstream's `registry.get(command)` name lookup, fed with ours `SelectedLoweringBindingIdentity::binding_identity()` (unrooted canonical name). This is name-based, not descriptor-based; ours already uses `registry.get(name)` in the same file. Flagged.
- `codegen/control_flow.rs` and `mod.rs` `release_compiled`: upstream's `registry.resolve_call(command, …)` is kept, but ours `invocation_surface_query()` replaces `own_surface_query()`. The `resolved.spec.name != command` spelling check stays as upstream wrote it. Flagged.
- `release_invoke_binding`/`release_binding_of`/`hold_release_binding*` key on command spelling: `release_binding(release, command, shape)`. Ported as is, with `word_is_dynamic` gating. Flagged.
- `cmd_subst.rs` `names_builtin` (`::cmd` → `cmd`) strips leading `::` textually. Ported; flagged.
- `command_binding.rs` `module_may_set_precision`: upstream text search for `tcl_precision` writers. Ported as a byte-window search over the source image, not a name-resolved variable proof. Flagged.
- `values.rs`/`control_flow.rs` `intern_compiled_local`/`intern_script_locals` and `bytecoded.rs` `is_compilable_local`: text-name based (see risky item 16).
- `const_subst.rs` `FoldedInvocation`: binding identities go through ours `CommandBindingIdentity::in_rooted_namespace(…).with_namespace_context(…)`. The naming system is preserved.

## Ported (the 8 hunks earlier recorded as dropped, revised per "port everything")
1. **`entered_command_surrogate` local interning** (upstream `applicable_codegen_binding` returning the probe's local table):
   - `bytecoded.rs::codegen_hook_locals` probes the registry hook's own emission in an isolated `CodegenCtx` and returns its LVT.
   - `values.rs::intern_entered_command_locals` interns each source local.
   - It is called after `entered_command_surrogate` returns a binding, both in `statements.rs::emit_call` and in `cmd_subst.rs::emit_generic_cmd_subst`.
2. **`info exists` `EXIST_ARRAY` shape:** `cmd_subst.rs::emit_inline_info_exists` plus `emit_original_array_exists_key`. An element of a compiled-local array emits `EXIST_ARRAY slot` after pushing the key.
3. **Catch-body errorInfo `trim_start`:** in `native_error.rs::emit_native_error_return`, ours' site for upstream's `emit_catch_error`. The no-text case uses `error_info_source_text` (the trailing-space form).
4. **`emit_incr` `amount_braced`:** the 4-argument `emit_incr`, threaded from `Statement::Incr`, `emit_inline_incr` and the tests (upstream's `, false` test edits included).
5. **`dispatch_under`:** a gate in `tcl_expr_eval.rs` `math_call` and `call` for the Tcl-table protocol, plus a test.
6. **Folder `declined` refactor:** `FoldOps.declined` plus `decline()`; it replaces `ambiguous`/`platform`, and `native_constant.rs` was updated. Upstream's reasons are kept: NumeralGrammar, Platform, Unspellable.
7. **Upstream's new and changed unit tests in `cmd_subst.rs`/`values.rs`:** 4 new tests and 5 edited tests ported, with internal names adapted and expected outcomes unchanged. One contradicting work test was removed (`qualified_head_word_stays_generic`), as upstream replaced it.
8. **`folded_literal_reads_apart` at the O129 fold:** it lives in `optimiser/propagation.rs`. Ours' O129 core is `fold_retained_builtin` there, not a tcl-registry `value_transfer.rs` site; the earlier note named the wrong file. Upstream added no `folded_literal_reads_apart` call in `value_transfer.rs` or sccp. Upstream's O116 hazard test is present.

Further upstream behaviour found by re-auditing every set-B file (upstream-added identifiers absent from the draft) and now ported:
- the `lset` append release fact (`BoundsRelease.lset_appends_at_length`) through `interval_bounds.rs`, `dataflow.rs` and the explorer
- `dict map` decline-path interning
- `string is` class resolution under the release (`string_.rs`)
- `precision_written` honoured by `text_of` for doubles

Merge-A reconciliation:
- No set-B file calls a `tcl_cmd_core::string` core other than `default_trim_set()` (unchanged signature), and none calls `index::drill`. Checked against both our drafts and upstream's set-B files. So no caller change is needed for the `numbers: NumberSyntax` and release parameters.
- `const_ops.rs` implements `NativeIndexOps`. Its methods match Merge-A's current resolved trait.

## Dropped upstream hunks
None. The remaining differences are upstream code shapes whose behaviour work already reproduces. Each has its proof:
- **Upstream `lower_inline_body`** (rebasing a lowered body to its source offset). Ours inline bodies are lowered at their original coordinates via `authored_inline_body_base`/`with_inline_body_operand`.
- **Upstream `StringEmission`/`emit_string_op` split.** It has a single caller upstream; ours inlines the same match. Ours `emit_inline_string_fqn_invoke` runs `entered_command subcmd …` unless the dialect registry names an implementation command address. That is upstream's "the command itself runs the subcommand" for releases without `::tcl::string::*`.
- **Upstream `emit_incr_local` `INCR_ARRAY1` path.** Ours `emit_incr` → `finish_increment(slot, array, …)` emits `INCR_ARRAY1(_IMM)` for an element of a local array.
- **`lseq` purity.** Ours already declares `PURE` with no expression fallback, matching upstream's measured 9.0.4/9.1.0 finding. Only the module and snippet prose differ.
- **`completion.rs` `has_return_options_dictionary`.** Ours selects the 8.4 legacy vs options grammar through `ReturnInvocationGrammar`, which encodes the same release fact.
