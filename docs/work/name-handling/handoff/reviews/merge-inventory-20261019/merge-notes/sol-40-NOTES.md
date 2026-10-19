# Sol-40 notes: compile/warning pass on the in-progress merge of origin/rust (fcb7927f0) into work

Nothing was committed, staged-reset, stashed or pushed. All fixes are unstaged
edits in the working tree on top of the staged merge resolution. No tests were
run (check builds only). Logs: `scratchpad/sol40/` (`c*.txt` root workspace,
`rt*.txt` runtime workspace, `wasm1.txt`, `wasi1.txt`, `vmwasm1.txt`).

## Fixes (file: what, why, whose intent)

### tcl-cmd-core
- `src/platform.rs`: `auto_path_installation`/`auto_path_environment` take a
  `PathReading` and pass it to `path::dirname`/`path::join`, whose signatures
  upstream made reading-aware. Callers in `rust/tcl-vm/src/interp.rs` and
  `runtime/rust/src/interp.rs` pass `PathReading::of_release(self.runtime_version())`.
  The work-side `~` expansion in `auto_path_environment` is unchanged.
  Intent: upstream's per-release path reading on work's auto_path code.
- `src/frame.rs` (new upstream file, `TclObjGetFrame` level words): `ops.as_str`
  no longer exists on work's `ValueOps`. It now reads `native_string_bytes`,
  strips `#` as bytes, and re-reads with `new_bytes`. A word whose bytes cannot
  be read is `BadLevel::Word` (fail-closed). Note: nothing calls `tcl_cmd_core::frame`
  yet (the runtime and tclvm keep work's `select_frame`); the module was kept,
  as an upstream feature.
- `src/lsort.rs`: `run_original` gains `release: TclVersion` and passes it to
  `prepare` (upstream added the release argument: BADSTRIDE from 8.6). Callers
  in tclvm `cmd_list.rs` and runtime `cmd_list.rs` pass `runtime_version()`,
  hoisted before the `&mut` borrow.
- `src/encoding.rs`: work's `SystemEncoding::Identity` was missing from upstream's
  `convert_from`/`convert_to`. `convert_to` copies the bytes (with Utf8).
  `convert_from` makes a lossy UTF-8 `String`, because the `String` result
  cannot carry non-UTF-8 bytes. The grammar (`encoding_named`) never produces
  Identity, so this is exhaustiveness only.
- Tests (`format.rs` `StrOps`: `as_str` replaced by `as_bytes`/`new_bytes`;
  `encoding.rs` and `path.rs` tests: `.message()`/`.error_code()` unwrapped):
  work's fallible `CmdError` accessors and byte-based `ValueOps`.

### tcl-vm
- `src/interp.rs` `settle_top_level`: used the `#[cfg(test)]` `command::opt_get`.
  It now uses work's checked `opt_get_checked(self, &options, b"-errorcode")`.
- `src/interp.rs` `current_lambda_term`: upstream looked the ProcDef up by
  `::<proc_name>` (a name lookup that cannot work for a lambda). It now reads
  the frame's own `native_procedure_binding` (else `native_procedure_execution`)
  declaration. Identity-based, per the naming system. The binding is still on
  the frame at the epilogue.
- `src/interp.rs` scripted-distribution `ProcDef` literal: `lambda_term: None`.
- `src/exec.rs` `Activation::new`: destructures `literal_values: _`. This is
  deferred port item 8: the field is not read until the literal-binding port.
- `src/compiled.rs` (`literal_values` field, `LiteralValues` struct and impl)
  and `src/subst.rs` (`literal_word_value`): marked
  `#[expect(dead_code, reason = "upstream literal binding; read once the deferred tclvm PUSH port lands")]`.
  This is clean-merged upstream code for not-yet-ported item 8. It was kept
  rather than deleted, and `expect` fails as soon as the port uses it.
- `src/cmd_chan.rs`: removed the unused `completion_from_cmd_error` import.
- `src/interp/native_name_queries/tests.rs`: `lambda_term: None`.
- Tests:
  - `cross_version_command_surface_e2e.rs` and `env_host_sync_e2e.rs`:
    `TclError.message` became `message_bytes()` (work's API).
  - `command_mutation_deopt_e2e.rs`: four `ScriptCompileTarget`s gain
    `evaluation: Compiled`.
  - `integer_argument_reads_e2e.rs`: `codegen_module(..).expect(..)`.
  - `variable_trace_semantics_e2e.rs`: upstream's expectation (bare
    `TCL READ VARNAME` / `TCL WRITE VARNAME`, as Merge-A ported) on work's
    mandatory-oracle helper `tclsh_output(script)`.
- `rust/tcl-vm-cli/src/main.rs` test: `repl_loop(.., true)` (upstream's new
  `interactive` argument; the test checks echoed results).

### tcl-compiler
- `cfg_builder/mod.rs` `lower_catch_dispatch`: removed the stale `!over_arity`
  term (its definition was gone). Upstream moved the arity refusal into
  lowering: `release_admits_arity` on `LoweringHookId::Catch`, so a
  bad-arity `catch` never becomes `Statement::Catch`. The comment says so.
- `cfg_builder/execution_regions.rs`: the `CommandBindingSite` gets a real
  `shape` from new `tokens_invocation_shape(registry, tokens)`. That is
  `InvocationShape::of(profile.vm_runtime_version, argv[1..], single-token
  Str/Esc words literal)`, mirroring lowering's `invocation_shape`/
  `seg_word_is_static_literal`. Not a `Default` shape, which would misbind
  `if`/`for`/`while`.
- `codegen/emitter/terminator.rs` `name_condition_command` and
  `codegen/mod.rs` `emit`/`emit_comment`:
  - `error_info_source_text` and `source_cmd_text` are work's `SourceImage`,
    while upstream's `ErrorInfoCommand.text` is `String`.
  - The text is taken via `try_text()`.
  - A non-UTF-8 command stamps nothing (fail-closed). The bytecode crate's
    `error_info_text` does the same.
  - The VM does not read `Instruction.error_info_command` yet (deferred
    items 4 and 8).
- `codegen/expressions.rs` `emit_prepared_expr_at_depth`: restored the final
  `self.emit_expr_at(node, 0)` for the non-positioned case. The merge had dropped
  it, leaving the function without a value.
- `compile_84.rs` `Segments::segment`: `command_at_time_script_with_config` now
  returns `Result<_, ParseCutUnavailable>`. When it is unavailable, the
  segmenter returns an empty script with no fatal tail. So there is no 8.4
  compile verdict, and the runtime's own native admission decides.
- `command_binding/runtime_entry.rs` and `lowering/native_control_carrier_tests.rs`
  (2 sites): `ScriptCompileTarget.evaluation: Compiled`.
- `compilation_unit.rs` test fixture: `top_level_evaluation: Compiled`.
- `compile_service.rs` START_CMD test and `tests/release_units.rs`:
  `source_cmd_text.try_text()`. `release_units.rs` also finds procedures by
  `FunctionAsm.name` through the new `named_procedure` helper, because the
  module keys procedures by `ProcedureId`, not by name.

### runtime/rust (tcl-runtime)
- `src/cmd_regex.rs`: imports `interp::obj_bytes` (used by the ported
  `regex_operands`/`compiled_glob`).
- `src/cmd_fs.rs` `path_result`: `CmdError::into_parts` is now fallible.
  It uses `interp.report_cmd_error(error)` (work's owner, with release-filtered
  codes).
- `src/interp.rs` `eval_script_file_completion`, `eval_compiled_completion`
  and `eval_stdin`:
  - They now return `Result<ScriptCompletion, NativeExecutionError>`, like
    work's `eval_completion`.
  - Each checks `native_execution_refusal()`.
  - `eval_stdin` stops at the first chunk refusal.
- `src/interp.rs` `eval_top_level_chunk`: rebuilt on work's
  `eval_str_boundary` protocol, plus upstream's level-0 settle.
  - Work's protocol: `NativeOperationScope`, `reset_outermost_native_error`,
    `clear_return_options_checked` (work's name for upstream's missing
    `clear_return_options`), currency checks, refusal propagation.
  - Upstream's settle: `settle_return`, then `unexpected_result`.
  - A Jim interpreter delegates to `eval_compiled_completion` (work's Jim
    source path).
- `src/interp.rs` removed three merge-orphaned methods; each has a work
  equivalent at its upstream call site:
  - `eval_proc_body`: work's procedure path sets `next_proc_body` inline.
  - `fire_exists_read_trace`: work's `info exists` fires read traces through
    `fire_var_traces_for` / `observed_variable_exists`, which reach the
    `::env` trace via `fire_var_trace_resolved`.
  - `release_arity`: work's `catch` checks per-release arity natively.
- `src/typed_value.rs`: removed the never-called `boolean`/`boolean_bytes`
  from the appended upstream classic section. Every upstream caller site
  uses work's native getters.
- `src/bytearray.rs`: the `ByteStringEncoding` import is `#[cfg(test)]`, as in
  work, because `binary_bytes` is test-only.
- `src/compiled_locals.rs`: `WordPart::Expression` (Jim `$(…)`) creates no
  locals.
- `src/cmd_package.rs` tests: removed the duplicate `TclVersion` import.
- `src/list/original_adoption_tests.rs`: `string_typed: false` in the two
  `TclList` literals.
- `tests/literal_index_encoding.rs` and `tests/tclsh_parity.rs`:
  `.expect("no host refusal")` on the now-fallible APIs.
- `examples/run_script.rs`: the stdin branch tested `Result::is_ok`, which
  swallowed a refusal as exit 0. It now returns 0 only for an `Ok` completion
  whose last chunk failed (upstream intent: it was already reported).

### Pre-existing work warnings (not caused by the merge; fixed because the task requires zero warnings)
Each item below was unchanged by the merge; HEAD had the same code and callers.
- `tcl-compiler`:
  - `taint.rs` test: unused `statement` loop binding.
  - `optimiser/expr_simplify.rs`: removed two unused `#[cfg(test)]` helpers
    (`collapse_*_expr_wrapper`).
  - `tests/support/body_identity.rs`: `#[allow(dead_code)]` on `fixture_id`.
    The shared support module is not used by every including test.
- `tcl-cshim/src/doors.rs`:
  - `provide_package`/`tell_the_engine` are `#[cfg(test)]` (only tests call
    them).
  - Test `let _ =` on `create_command`, keeping the old implicit discard.
- `tcl-engine-wasm/src/session.rs`: `define_command` is `#[cfg(test)]` (only
  tests call it).
- `tcl-lsp-core/src/formatting/keywords.rs`: `#[expect(dead_code)]` on the
  test-only `RangeTable::SubcommandOptions` variant.
- `tcl-spectcl/src/discovery.rs`: removed the unused test wrapper
  `collect_beside_manifests`. Its doc moved to `collect_beside_manifests_observed`.
- `xtask/src/editor_extensions.rs`: removed the unused test wrapper
  `verify_every_language_activates`. Its doc moved to
  `verify_language_activation_manifest`.

### Formatting
`cargo fmt --all` was run in the root workspace and in `runtime/rust`. Only 33
files changed, and all of them were touched by the merge or by this pass.

## Compile-pass deferrals / items for the coordinator
- Deferred item 8 (tclvm literal binding): `LiteralValues` /
  `literal_word_value` / `CompiledUnit.literal_values` are `#[expect(dead_code)]`.
  Remove the attributes when the port reads them.
- `tcl_cmd_core::frame` (upstream `TclObjGetFrame` core) compiles but has no
  caller. Both engines use work's frame selection. It would be wired by
  whoever reconciles frame-level parsing.
- `ErrorInfoCommand.text` stays `String` (upstream). Work's command text is a
  byte `SourceImage`, so non-UTF-8 commands get no errorInfo stamp. Making it
  `SourceImage` belongs with deferred items 4 and 8, when the VM starts
  reading it.
- runtime `info exists`: upstream's `fire_exists_read_trace` restored `exc`
  after a swallowed trace error. Work's path drops the pending message but
  does not restore `exc`. To verify with the env-sync tests: `info exists env(X)`
  under an erroring read trace.
- `tcl-vm-cli` `repl_loop`: the merged code wraps `eval_interactive` in
  `Ok(..)`, so host refusals are reported only via the completion. Behaviour
  as merged; unchanged here.
- `runtime/rust/src/cmd_list.rs`: there is a doubled doc line above `lsort`
  (merge artefact). Cosmetic; left as is.

## Final check results
- Root workspace (`CARGO_INCREMENTAL=0 cargo check --workspace --all-targets --all-features`):
  0 errors, 0 warnings (`sol40/c14.txt`).
- Runtime workspace (`runtime/rust: cargo check --all-targets --all-features`):
  0 errors, 0 warnings (`sol40/rt2.txt`).
- `rust/tcl-lsp-server-wasm` (`cargo check --target wasm32-unknown-unknown --all-targets`,
  CC from `scripts/dev/wasm-cc-env.sh`): 0 errors, 0 warnings.
- `rust/tcl-lsp-server-wasi` (`cargo check --target wasm32-wasip1 --all-targets`):
  0 errors, 0 warnings.
- Extra: `rust/tcl-vm-wasm` (`--target wasm32-unknown-unknown --all-targets`):
  0 errors, 0 warnings.
- The `wasm32-unknown-unknown` and `wasm32-wasip1` rustup targets were added
  for these checks (only `wasip2` was installed).
- The wasm checks used `CARGO_TARGET_DIR=<repo>/target`.
- `cargo fmt --all -- --check`: clean in the root workspace and in
  `runtime/rust`.
- `rust/tcl-vm-wasm/Cargo.lock`: `cargo check` added 5 missing packages
  (`bytecount`, `cc`, `find-msvc-tools`, `shlex`, `tcl-host-c-abi`). These
  are deps the merged tree needs, which the regenerated lockfile lacked.
  Additions only; left unstaged for the coordinator to review. The
  `tcl-lsp-server-wasm` and `tcl-lsp-server-wasi` lockfiles were unchanged.
- Not run: clippy (CI's gate) and any tests.
