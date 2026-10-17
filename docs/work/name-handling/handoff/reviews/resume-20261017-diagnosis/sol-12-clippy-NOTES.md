# Sol-12: clippy gate for the LSP packages

Repo `/home/user/tcl-lsp`, branch `work`, base `e46e88617`. Nothing is committed. The edits are uncommitted in the tree.

Gate command:
`cargo clippy --locked -p tcl-lexer -p tcl-compiler -p tcl-registry -p tcl-lsp-core -p tcl-lsp-db -p tcl-lsp-server --all-targets --keep-going -- -D warnings -A clippy::assert_is_empty`

## Status: the gate is NOT clean (exit 101)

- Final gate log: `/home/user/tcl-lsp/tmp/logs/clippy-sol12-9.log`, exit 101.
  - The only errors are two `too_many_arguments` errors in `rust/tcl-runtime-api/src/native_literal.rs`: `register_using` has 8/7 arguments and `create_using` has 9/7. See Blockers.
- I also ran the gate with `-A clippy::too_many_arguments` added, only to see what comes after that blocker. This was a probe, not a fix.
  - Log: `/home/user/tcl-lsp/tmp/logs/clippy-sol12-10-probe.log`
  - Clean: `tcl-lexer` (all targets), `tcl-syntax`, `tcl-cmd-core`, `tcl-registry` (lib and tests), `tcl-host-c-abi`, `tcl-test-support`.
  - **`tcl-compiler` lib: 736 errors. `tcl-vm` lib: 349 errors.** Between them, 227 are `too_many_lines`: 160 in the compiler and 67 in the VM, median 117 lines per function, worst 308.
  - The rest include about 100 `redundant_closure`, 60 `collapsible_if`, 53 `needless_pass_by_value`, 46 `items_after_statements`, 43 `used_underscore_binding`, 41 `type_complexity`, 41 `useless_conversion`, 25 `borrow_as_ptr`, 22 `too_many_arguments`, 12 `mutable_key_type` and 12 `large_enum_variant`.
  - Not checked yet, because they sit behind the compiler and VM: the compiler and VM test targets, `tcl-lsp-core`, `tcl-lsp-db`, `tcl-lsp-server`, `tcl-engine-*` and `tcl-pkg-model`.
- I tried `cargo clippy --fix` (machine-applicable suggestions only) on `tcl-compiler`. The result did not compile:
  - About 50 `E0603` errors: the redundant-closure method paths it wrote go through private modules.
  - Several `FnOnce` "not general enough" errors.
  - It also applied `assert_is_empty` rewrites, which the gate allows on purpose.
  - I reverted all of it. `tcl-compiler` is unchanged from HEAD.
- `cargo fmt --all --check` passes.
- Build check: `cargo build --locked -p tcl-lsp-core -p tcl-lsp-server --lib` exits 0 (`/home/user/tcl-lsp/tmp/logs/build-sol12.log`).
- I did not run the test suites.
- Disk: 5.6 GB free at the end.

## Blockers needing a decision

1. **`tcl-runtime-api` `NativeLiteralWorld::register_using` (8 args) and `NativeLocalNameTable::create_using` (9 args).**
   - Fixing this needs a public signature change. I had bundled `check`/`refuse`/`retain` into a `NativeLiteralGuards` struct, which meant editing the call sites in `tcl-vm` and `runtime/rust`.
   - Your instruction allows no API changes in the runtime crates, so I reverted that. Allowing the lint is also not permitted.
   - This needs an owner decision: either accept the API change (the guards struct is mechanical) or grant an exception.
2. **The size of `tcl-compiler` and `tcl-vm`.** This needs splitting into parallel work: about 1,085 diagnostics, including 227 function splits. The VM half also falls under the paused-migration rule ("strictly lint-mechanical"); function splitting is lint-driven but it is not mechanical.

## VM/runtime-crate changes (`tcl-runtime-api`, `tcl-cmd-core`; `tcl-vm` and `runtime/rust` are untouched)

Every change here is lint-driven and behaviour-preserving. None changes a public signature.

- **Reverted after your instruction:**
  - the `NativeLiteralGuards` API, its test call sites and its `tcl-vm`/`runtime/rust` callers;
  - the use of type aliases in `tcl-vm`/`runtime/rust` implementors;
  - the `runtime/rust` call-site change to `c86_list_insertion_steps(&index)` (the registry signature is by-value again).
- **Public items added:** none. Both type aliases are private and are only used inside the trait default method.

### tcl-runtime-api
- `src/lib.rs`
  - `type_complexity` on `NativeIntrospectionOps::proc_body_original_value`: a private `type ProcBodyValueResult<V,E,O>` alias. The expanded type is identical.
  - `doc_markdown`: backticks.
- `src/native_literal.rs`
  - `err_expect`: `after.err().expect(..)` became `after.expect_err(..)`.
  - The `too_many_arguments` errors are left as they are (Blocker 1).
- `src/command_inventory.rs` — `redundant_closure`: `.map(AsRef::<[u8]>::as_ref)`.
- `src/variable_destruction.rs` — `doc_markdown`.

### tcl-cmd-core
- `clock.rs` — `chunks_exact_with_const_size`: `args[1..].as_chunks::<2>().0`. The remainder is ignored, as before.
- `encoding/original_names.rs`
  - `needless_pass_by_value`: the private `read_error` takes `&HostError`.
  - `too_many_lines` in `prepare_source_file`: the decoding step moved to a private `decode_source_script`.
- `index.rs`
  - `type_complexity`: a private alias `IndexBinarySelection<R>` in the trait default.
  - `map_unwrap_or`: `map_or`.
  - `doc_markdown`.
- `list.rs` — `empty_line_after_doc_comments`: removed an orphan doc line that documented no item.
- `lsearch.rs`
  - `too_many_lines` (`parse_options`, `lsearch_with`): the per-release option tables moved to module consts. New private helpers: `select_option`, `parse_stride`, `parse_index_path`, `stride_key_path`, `search_number`. This also fixes `items_after_statements` and `single_match_else`.
  - `stride_key_path` replaces the `g as usize` cast with `usize::try_from`. On 64-bit targets the result is the same.
  - `too_many_arguments`: the private `result_one`/`result_all` take `&SearchInput` (the fields are the same values).
  - `useless_conversion`.
  - `semicolon_if_nothing_returned`.
- `lsort.rs`
  - `collapsible_if` ×2: `if let … &&` chains.
  - `semicolon_if_nothing_returned`.
  - `needless_pass_by_value`: the local `merge` takes slices.
  - `manual_flatten`.
- `native_append.rs` — `too_many_arguments`: a private `AppendOperands { receiver, source }` struct passed to the two private helpers.
- `native_boolean_truth.rs`
  - `unnested_or_patterns`.
  - `needless_pass_by_value`: the private `legacy_result_probe` takes `Option<&NativeScalarCache>`.
  - `doc_markdown`.
- `native_cat.rs` — `type_complexity`: a private `JimCatCheck` alias on private functions.
- `native_list_index.rs`
  - `collapsible_if`.
  - `useless_conversion` ×2: dropped `.into()` on `CmdError`.
  - `doc_markdown`.
- `native_list_numeric.rs` — `float_cmp`:
  - `search_order` uses `a.partial_cmp(&b).unwrap_or(Greater)`.
  - The Jim `sort_order` uses `unwrap_or(Less)`.
  - Both give exactly the old outcomes, including NaN and ±0.
- `native_option_index.rs` — `needless_lifetimes`: `impl Path<'_>`.
- `regex.rs`
  - `too_many_lines` ×3: private helpers `jim_adjusted_start` (shared with `jim_byte_offset`), `regexp_about`, `about_answer`, and a private `UNMATCHED` const.
  - `useless_conversion`.
- `string.rs`
  - `must_use_candidate` ×2: `#[must_use]` added (the signature is unchanged).
  - `too_many_lines` in `replace`: split into private `replace_jim` and `replace_c`.
  - `doc_markdown`.
- Only `doc_markdown` backticks: `native_jim_index.rs`, `native_numeric.rs`, `return_options.rs`, `string/native_c.rs`, `var.rs`.

## Other crates

### tcl-lexer
- `word_parts/cursor.rs`
  - `cast_possible_truncation` ×13, all in tests: a test-only `offset()` helper using `u32::try_from(..).expect`. The values are unchanged.
  - `too_many_lines`:
    - in `word_cursor_prefix_in`: moved to `empty_boundary_prefix`;
    - in `prefix_at`: moved to `index_prefix` plus a private `IndexPrefix` enum.
  - `doc_markdown`.
- `word_parts.rs` — `too_many_lines` in `scan_parts`: `ComponentParseError::spanned_part`.
- `structural_index.rs` — `too_many_lines` in `scan_script`: `separates_closed_word`.
- `native_script_words.rs`
  - `too_many_lines`: `rebase_fatal_tail`, `rebase_source_extent`, `rebase_terminator`.
  - `needless_pass_by_value`: derived `Clone, Copy` on the private `SyntaxValidation`.
  - `doc_markdown`.
- `parse_cut.rs`
  - `ok().is_some_and` became `is_ok_and`.
  - `too_many_lines` in `drain`: `component_cut` and `source_geometry`.
- `script_completeness.rs` — `redundant_closure_for_method_calls`.
- `jim_script_tokens.rs` — `doc_markdown`.

### tcl-host-c-abi
- `lib.rs`
  - `useless_conversion`: `i64::try_from(c_long)` is the same type on 64-bit. It is now a private generic `widen_c_long<Long: Into<i64>>`, which is lossless for both 32- and 64-bit `c_long`.
  - `doc_markdown`.

### tcl-syntax
- `expr/substitution.rs`
  - `too_many_lines`: `token_word_admits`.
  - `semicolon_if_nothing_returned`.
- `naming.rs` — `double_must_use`: removed a redundant `#[must_use]` on an `impl Iterator` return.
- `naming/native.rs`
  - `collapsible_if`.
  - `doc_markdown`.
- `scalar_getter.rs`
  - `unused_self`: `let Self(()) = self;` consumes the target-proof token, so it is still required.
  - `match_same_arms`: merged arms. The moved pattern matches no arm in between.
  - `doc_markdown`.
- `scalar_getter/errors.rs` — `match_same_arms`.
- Only `doc_markdown`: `naming/jim_enumeration.rs`, `naming/reporting.rs`, `naming/tcloo_link.rs`, `native_boolean_truth.rs`, `native_equality.rs`, `native_jim_lookup.rs`, `native_tcl_utf.rs`, `native_variable_name.rs`, `number.rs`.

### tcl-test-support
- `encoding_utf8.rs` — `chunks_exact_with_const_size` (`as_chunks`) and `doc_markdown`.

### tcl-registry
- `commands/tcl/dict.rs`
  - `match_like_matches_macro`/`single_match`: an `if` with early return.
  - `match_same_arms`: `Some(V8_5) | None`.
- `commands/tcl/namespace_.rs` — `double_ended_iterator_last` and then `filter_next`: `.rfind(..)`.
- `commands/tcl/oo_define.rs` — `needless_question_mark`.
- `commands/tcl/return_.rs` — `too_many_lines` in `spec()`: the option table moved to `const OPTIONS: &[OptionSpec]`.
- `const_fold.rs` — `similar_names`: `list` renamed to `source`.
- `definer.rs` — `too_many_lines`: the wrapper arm moved to a private `source_wrapper_indices_where` plus a private `SourceMemberQuery` struct.
- `intrinsic.rs` — `match_same_arms`.
- `native_compilation.rs` — `too_many_lines` ×2: `namespace_which_preserves_names` and `generic_before_or_in_script`.
- `native_jim_lookup.rs`
  - `too_many_lines`: six inline Jim helper bodies (`which`, `code`, `origin`, `delete`, `upvar`, `info`) moved to **new byte-identical** asset files under `native_scripts/` and loaded with `include_str!`. This is the existing pattern.
  - New files: `jim_namespace_{which,code,origin,delete,upvar,info_source}.tcl`, all untracked. Please `git add` them.
  - I checked that each file's bytes equal the old raw string.
  - `needless_raw_string_hashes`.
- `native_jim_lookup/namespace_subcommands.rs` — `items_after_statements`: statics moved before the statement.
- `native_list_operations_compilation.rs`
  - `similar_names`: renamed to `prefix_last`/`suffix_first`.
  - `needless_pass_by_value`: a private consuming `NativeListInsertionIndex::immediate(self)`. The public signature is unchanged.
- `native_numeric_error/command.rs` — `match_same_arms` ×2: unguarded arms merged at their later positions. The moved patterns match no guarded arm in between.
- `native_property_lookup.rs` — `enum_glob_use`: explicit variant imports.
- `native_scripted_distribution.rs` — `redundant_closure_for_method_calls` ×2.
- `registry.rs`
  - `single_match_else`.
  - `unused_self`: the private `selected_argument_role_assignments` became an associated function and lost its unused parameters.
  - `large_types_passed_by_value` ×2: `&ResolvedInvocation`.
- `resolved_invocation.rs`
  - `needless_lifetimes`.
  - `must_use_candidate` ×4.
  - `items_after_statements`.
  - `too_many_lines` ×2: `nul_free_literal_words` and `sole_world_effect_descriptor` (by reference); `selected_taint_transform`, `selected_arity`, `selected_procedure_definition`.
- `source_name_ownership.rs`
  - `collapsible_if`.
  - `semicolon_if_nothing_returned`.
  - `too_many_lines`: `nominal_alias_replacement`.
- `spec.rs`
  - `nonminimal_bool`: a named `readable_boundary` (De Morgan, same short-circuit order).
  - `too_many_lines`: `skip_clause_selector`.
- `state_transition.rs`
  - `unnested_or_patterns` ×2.
  - `single_match_else`.
- `value_transfer/completion.rs` — `as_chunks`.
- `value_transfer/const_ops.rs`
  - `match_like_matches`/equality: an `if` with early return.
  - `needless_pass_by_value`: the private `decline_execution` takes `&NativeExecutionError`.
  - `ignored_unit_patterns`.
- `value_transfer/reference_body.rs` — `collapsible_if`.
- Only `doc_markdown`: `channel_configuration.rs`, `commands/tcl/exists_.rs`, `commands/tcl/info_jim.rs`, `dialects.rs`, `model/context.rs`, `native_alias_registration.rs`, `native_array.rs`, `native_command_creation.rs`, `native_command_literal.rs`, `native_dictionary.rs`, `native_error_log.rs`, `native_index_lookup.rs`, `native_introspection_compilation.rs`, `native_namespace_name.rs`, `native_tcloo_method_definition.rs`, `native_try.rs`, `native_usage.rs`, `native_variable_destruction.rs`, `special_vars.rs`.

## Logs
- Waves: `/home/user/tcl-lsp/tmp/logs/clippy-sol12-{1..9}.log`
- Probe: `/home/user/tcl-lsp/tmp/logs/clippy-sol12-10-probe.log`
- Reverted fix attempts: `/home/user/tcl-lsp/tmp/logs/clippy-sol12-fix{1..4}.log`
- Build: `/home/user/tcl-lsp/tmp/logs/build-sol12.log`
- Snapshot of the diff before the `--fix` attempts: `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/sol12-pre-fix2.patch`

## Not verified
- I did not compile `runtime/rust` (it has its own lockfile, and disk is limited). It is unchanged, and every public API it uses is unchanged.
- I did not run any tests.
