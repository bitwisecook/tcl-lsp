# Sol-44: every interpreter is a native-core interpreter

Branch `work`, base HEAD bf3bc3b5e2. Nothing is committed, staged, stashed or reset.
`sol-44.patch` is `git diff` over the paths listed in "Patch scope" below.
The other changes in the tree are not mine: `rust/tcl-compiler/src` and `rust/tcl-registry`
belong to Sol-45, and `docs/design/contracts/name-handling.md` belongs to another agent.

User decision (`user-decisions-20261020T0230Z.json`, "Require with_native_core"): every harness and
embedder builds its interpreter with `Vm::with_native_core(...)` and an explicit profile.
`Vm::new`/`Vm::with_output` become test-only.

One accident to report: while removing temporary probes, I ran `git checkout -p --` with stdin
from `/dev/null`. It reached its first hunk prompt (Cargo.lock), read EOF, and exited without
discarding anything. I checked `git status` and the Cargo.lock diff right afterwards: every
change in the tree, mine and other agents', was still there.

## 1. Constructor migration (task 1)

### tcl-vm API (compiler-enforced)
- `Vm::new()`, `Vm::with_output()`, `Vm::with_shared_output()` and `impl Default for Vm` are now
  `#[cfg(test)]` and `pub(crate)`. No caller outside tcl-vm's own unit tests can build an
  interpreter without a native core.
  - Inside unit tests, `Vm::new()`/`with_output()` are shorthand for `test_support::native_vm*`.
  - The old engine-less constructor is still available to unit tests as
    `Vm::without_native_engine()` / `without_native_engine_with_output()`. Only the 11 unit tests
    that exercise the unselected-engine state use it (listed below).
- New `pub fn tcl_vm::standard_host()`: `NativeHost`, or `BrowserHost` on
  wasm32-unknown-unknown. Embedders that have no host of their own pass it to `with_native_core`.
- New module `tcl_vm::test_support`, gated `#[cfg(any(test, feature = "test-support"))]`. It is
  the shared one-line harness constructor:
  - `native_vm()` / `native_vm_with_output(out)`: the Tcl 9.0 native core, pinned to the
    permissive fallback profile (`DialectProfile::plain_tcl()`). Modules lowered against
    `CommandRegistry::build_default()` / `BytecodeCompileService::default()` are compiled for that
    profile, so this matches what the old `Vm::new()` admitted. Before 6fa086c780, `Vm::new()` was
    a fallback pin whose engine was the release's own.
  - `native_vm_for_release(TclVersion, out)`, `native_vm_for(env, out)`,
    `native_vm_for_profile(profile, out)`: the native core of the named profile, pinned to it.
    They panic for a profile with no native core (vendor profiles such as irules or bpf; those run
    on a host core plus `set_dialect_profile`).
  - `profile(env)`, `DEFAULT_ENVIRONMENT`.
- tcl-vm's integration tests and examples reach `test_support` through a self dev-dependency:
  `tcl-vm = { path = ".", features = ["test-support"] }`. Cargo.lock gains that one line.
- `thread::create` / `tpool` workers (production code) used to call `Vm::with_output`, so every
  worker refused every command name. A worker is now a native-core interpreter of the parent's
  actual engine, with the parent's core bootstrap inputs:
  - `Shared.native_core`, filled by `Vm::native_core_recipe()`;
  - `Vm::with_native_core_recipe()` builds the worker.
- Child interpreters (`interp create`) now configure their error-object stack for their engine.
  Without that, an error inside a safe child refused with `native error-context owner`.
  This was found by the migration: safe_interp_e2e went from 5/13 to 9/13.

- Smaller VM edits:
  - `value.rs`: the `TCL_LSP_TRACE_RETIRED_HEADER` backtrace aid is also compiled under
    `feature = "test-support"`, so integration tests can use it.
  - `interp.rs`: a collapsible `if` that clippy flags is collapsed (`begin_instruction` site).

### Callers migrated
- tcl-vm unit tests (src): unchanged call sites. `Vm::new()` now builds the native core.
  These 11 tests that assert the engine-less state switched to `Vm::without_native_engine()`:
  - cmd_binary `binary_private_tokens_follow_the_actual_bootstrap_release`
  - cmd_coro `coroutine_original_name_lookup_keeps_missing_and_unavailable_distinct`
  - cmd_info `original_info_loaded_keeps_physical_result_owner_separate_from_source_labels`
  - cmd_oo abstract_tests `original_abstract_public_rooted_lookup_joins_canonical_token_and_registry_marker`
  - command `original_legacy_bareword_profile_labels_do_not_issue_native_function_or_error_owners`
  - command `physical_procedure_capture_and_header_remain_separate_from_authored_names`
  - interp `logical_local_compiler_is_explicit_retained_and_independent_of_host`
  - interp `explicit_host_engine_keeps_source_logical_and_availability_axes_separate`
  - interp `source_word_recipe_requires_its_own_provider_and_cache_identity`
  - native_command_names `origin_string_result_requires_actual_native_core_issuer`
  - native_variable_inventory_policy_tests `unsupported_inventory_policy_refuses_before_a_pattern_getter`
- tcl-vm integration tests and examples: every `Vm::new/with_output/default` now goes through
  `test_support`.
  - A harness that pinned a release or profile right after constructing now builds that
    profile's core: `native_vm_for_profile/for_release`, and the redundant pin is dropped.
    Vendor-profile sites (irules, bpf, the profile loop in
    `cross_version_command_surface_e2e::the_profile_form_of_a_pin_is_the_context_the_profile_names`) keep the 9.0 core plus
    `set_dialect_profile`.
  - `cmd_thread_e2e`'s irules worker test builds on its 8.4 host core.
  - Sol-41's inline `with_native_core` blocks in 23 harnesses now call `native_vm_for_profile`.
  - `opcode_c_parity`: `let (_, c) = run_fresh(..)` dropped the VM before reading `c`. It is now
    `let (_vm, c)`; see finding F2.
  - `examples/eval.rs` evaluates through the VM's own compile entry (`try_eval_source`). Its
    standalone module path is cluster B.
- Harnesses switched to the VM's own compile entry (`vm.try_eval_source(src)` in place of
  `lower_to_ir`/`codegen_module`/`run_module`), kept only where that measured better:
  - cmd_control_e2e `run`
  - namespace_surface_e2e `run_at`/`run_flipping`
  - cmd_math_expr_e2e `run`
  - cmd_coro_e2e `run`

  I tried the same switch on 13 more harnesses. It traded cluster B for cluster C (builtins
  100→80, run_script 74→52, cmd_event 13→10, safe_interp 5→4, cmd_oo 59→55), so I reverted it.
- No expectation was changed anywhere.

### Embedders and other crates (task 2: constructor edits only)
| crate | file | change |
|---|---|---|
| tcl-engine-tclvm (mine) | src/lib.rs `TclVmEngine::with_registry` | `with_native_core(stdout, standard_host(), tcl9.0, default inputs)`; was `Vm::new()` + pin + `set_native_engine_profile` |
| tcl-engine-tclvm | src/lib.rs tests | `tcl_vm::test_support::native_vm*` |
| tcl-vm-wasm (mine) | src/lib.rs `run` | `with_native_core(tcl9.0)` + fallback pin (it compiles with `BytecodeCompileService::default()`) |
| tcl-debugger | src/backend.rs | `with_native_core(tcl9.0)`; a missing core is `DebugError::Failed` |
| tcl-fuzz | src/wasm_diff.rs (direct arm), src/generator.rs (test) | `with_native_core(tcl9.0)` + fallback pin |
| tcl-irule-test | src/live.rs tests, src/pure_functions.rs tests; Cargo.toml dev-dep feature `test-support` | `test_support::native_vm*` |
| tcl-spectcl | tests/codegen_stamps.rs, tests/pack_source_e2e.rs; Cargo.toml dev-dep feature `test-support` | `test_support::native_vm*` |
| tcl-compiler (Sol-45's crate, one site) | tests/wasm_codegen.rs `capture_native_refusal_entry` | `Vm::default()` + pin became `with_native_core(profile)` (no Cargo.toml change) |
| tcl-vm-cli | none | Sol-41 had already made it native-core |

xtask, tcl-cshim, tcl-lsp-db, tcl-spec-hooks, f5-cli, tcl-cli, tcl-mcp and bpf-tcl construct no
`Vm`; tcl-spec-hooks goes through `TclVmEngine`. A behaviour change for every `TclVmEngine`
consumer: it is now a core-bootstrap interpreter, without the distribution globals `argv`,
`argv0`, `argc` and `tcl_interactive`. For Tcl profiles the scripted distribution libraries are
Jim-only, so nothing else is lost.

Gates:
- `cargo check --tests` is clean, with 0 warnings, for tcl-vm (+examples), tcl-engine-tclvm,
  tcl-vm-cli, tcl-irule-test, tcl-fuzz, tcl-debugger, tcl-spectcl, tcl-compiler, tcl-cshim,
  tcl-lsp-db, tcl-spec-hooks and xtask.
- tcl-vm-wasm was checked for the host target only; the wasm32 build was not run.
- `cargo fmt --check` is clean on every crate I touched.
- clippy: only tcl-vm lib, filtered to my changed lines; see Gaps.

## 2. Re-measurement (task 3)

Each binary was built and run on its own or in small groups; the executables were deleted
afterwards. "CI before" is the failure count in ci-5201 (shard 2 is missing from those lists).
"Ctor only" means the constructor migration alone. "Final" is the harness as left in the tree.

Pass/total. "CI fail" is the ci-5201 failure count. CI ran before Sol-41's commit, so for the
binaries Sol-41 already migrated (marked †) the HEAD-before number is Sol-41's "after"; where it
is not stated, it is 0 (cluster A). "—" means not measured or not listed.

| binary | CI fail | ctor only | final (tree) | main remaining clusters |
|---|---|---|---|---|
| tcl-vm lib (unit) | — (shard 2 missing) | HEAD measured 945/1387 | **1107/1387** (0 new failures; 162 fixed) | C 27, cmd_oo method preparation "genuine native expression provider" 15, others |
| builtins_e2e | 126 | 100/126 | **100/126** | B 18, other 8 |
| namespace_surface_e2e | 123 | 0/123 (B) | **108/123** (VM entry) | other 12, C 2 |
| cmd_control_e2e | 114 | 0/114 (B) | **61/114** (VM entry) | C 49 |
| cmd_math_expr_e2e | 102 | 93/102 | **95/102** (VM entry) | C 5, V 1 |
| run_script | 98 | 74/98 | **74/98** | B 19, other 5 |
| opcode_c_parity | 89 of 90 | 15/90 | **54/90** (`_vm` kept alive) | H 16, S 8, C 5, other 6 |
| command_mutation_deopt_e2e | 84 | **32/84** | 32/84 | B 27, C 6, P 5, other 13 |
| cmd_oo_e2e | 73 | aborts (O) | **59/71** + 2 stack-overflow (O) | B 5, other 7 |
| embed_api_e2e | 26 | **22/26** | 22/26 | B 1, other 3 |
| c_error_paths_e2e † | 20 | **15/20** | 15/20 | C 4, V 1 (`lsort -command nosuch`) |
| opcode_catch_parity | 17 | **10/17** | 10/17 | H 7 |
| uplevel_coro_e2e † | 15 | **15/15** | 15/15 | |
| release_number_semantics_e2e † | 14 | **6/14** | 6/14 | C 3, 8.4 numeric model 5 |
| variable_trace_semantics_e2e † | 13 | **16/26** | 16/26 | C 3, trace semantics 7 |
| tricky_resolution_e2e | 13 | **13/13** | 13/13 | |
| cmd_event_e2e | 13 | **13/13** | 13/13 | |
| safe_interp_e2e | 12 | 5/13 | **9/13** (child error stack) | B 3, E 1 |
| coro_error_trace_e2e † | 8 | **8/8** | 8/8 | |
| tailcall_uplevel_e2e † | 7 | **7/7** | 7/7 | |
| step_trace_inline_e2e | 7 | **4/7** | 4/7 | C 1, other 2 |
| mathfunc_derivation_e2e | 7 | **2/9** | 2/9 | B 7 |
| return_code_boundary_e2e † | 5 | **5/5** | 5/5 | |
| error_info_text_e2e † | 5 | **5/5** | 5/5 | |
| command_traces_e2e | 5 | **5/7** | 5/7 | other 2 |
| cross_version_info_surface_e2e † | 4 | **3/6** | 3/6 | C 3 |
| cross_interp_alias_e2e | 4 | **5/6** | 5/6 | V 1 |
| variable_name_resolution_e2e | 3 | **4/7** | 4/7 | C 1, other 2 |
| rename_return_source_e2e † | 3 | **2/3** | 2/3 | "original C86 split worker" refusal |
| word_value_resolution_e2e | 2 | **1/3** | 1/3 | B 2 |
| variable_container_models | 2 | **0/2** | 0/2 | Jim dictionary-sugar refusal, 1 of 60 vectors |
| legacy_variable_traces_e2e | 2 | **4/4** | 4/4 | |
| cross_version_escapes_e2e | 2 | **1/3** | 1/3 | B 2 |
| tclsh_parity_e2e † | 1 | **1/2** | 1/2 | C / compiled-vs-direct |
| release_running_bytecode_e2e † | 1 | **0/1** | 0/1 | C, item 2 |
| literal_index_encoding_e2e † | 1 | **1/2** | 1/2 | C, item 8 |
| cross_version_lset_e2e † | 1 | **2/2** | 2/2 | |
| package_coro_e2e † / package_diagnostics_e2e † | — | — | **12/12 / 5/5** | |
| cmd_collections_e2e | — | **60/67** | 60/67 | B 4, other 3 |
| cmd_coro_e2e | — | 0/59 (B) | **34/59** (VM entry) | C 24, V 1 |
| language_e2e | — | **30/39** | 30/39 | B 9 |
| namespace_colon_runs_e2e | — | **21/22** | 21/22 | other 1 |
| regexp_cmd_corpus / command_resolution_conformance | — | 0/1 / 0/1 | 0/1 / 0/1 | B (C via entry) |
| cross_interp_reentry_e2e | — | **1/2** | 1/2 | B 1 |
| word_parts_owner_e2e | — | **4/4** | 4/4 | |
| expr_surface_e2e | — | — | 0/2 | "borrowed error-code bytes" value refusal (VM gap) |
| cross_version_expr_surface_e2e | — | — | 0/6 | B 6 |
| tcl-vm-cli bin (tclvm unit) | 10 | **8/9** | 8/9 | `{*}`/lassign under `--tcl-version` (C) |
| tcl-vm-cli script_errors | 5 | **3/5** | 3/5 | 8.5+ nested direct levels; `if {1} {break}` |
| tcl-vm-cli top_level_e2e | — | **6/10** | 6/10 | top-level coroutine (item 9), 8.4/8.5 direct substitution levels, 8.5 stdin errorInfo, 8.5 `return -code error` file trace |
| tcl-vm-cli channel_output | — | **10/12** | 10/12 | thread worker "native compiler preflight provider required" (B/C) |
| tcl-vm-cli static_extensions | — | **2/2** | 2/2 | |
| tcl-spectcl eval_loader | 24 of 28 (task brief) | **24/28** | 24/28 | snapshot drift (eda_microchip), notice layout, 2 others |
| tcl-spectcl codegen_stamps | 28 | **1/29** | 1/29 | B 22, "CompileService does not expose entry-aware byte parsing" 3 |
| tcl-spectcl pack_source_e2e | — | **3/6** | 3/6 | other 3 |
| tcl-engine-tclvm lib | — | **35/51** | 35/51 | C, `ScriptBytes` vs `Script` message variant, "native resident storage identity is unavailable", 1 test needing an engine-less VM (F5) |
| tcl-irule-test lib | 35 | **29/59** | 29/59 | all 30: "bytecode manifest disagrees with the runtime on environment: compiled for tcl9.0, runtime holds f5-irules" (LiveSession; pre-existing, not the constructor) |
| tcl-debugger lib | 10 | **7/17** (+1 hang) | 7/17 | B ("native compiler preflight provider required" at launch); `launch_survives_deeply_nested_control_flow` hangs >30 min inside tcl-compiler lowering (`prove_possible_bodies` → `stamp_original_tokens`, Sol-45) |
| tcl-fuzz bin | 11 | **93/106** | 93/106 | wasm_diff 8 + generator 1 (B: the direct arm runs a standalone module), bpf_diff 2 (BPF001), wasm::tests 2 (local wasmtime instantiation, environmental) |


After the item-4 VM changes (section 4), I re-ran these with no change from the table:
- tcl-vm: c_error_paths, error_info_text, builtins, cmd_math_expr, cmd_control, run_script,
  release_number_semantics, coro_error_trace, uplevel_coro, tailcall_uplevel,
  return_code_boundary, safe_interp, cross_interp_alias, variable_trace_semantics, package_coro,
  package_diagnostics;
- tcl-vm-cli: script_errors, top_level_e2e, bin;
- tcl-engine-tclvm lib.

### Clusters still failing (tags)
- **B, standalone module compile**: `NativePreflightRequired` from `run_module` of a module the
  harness compiled without an entry (`lower_to_ir`+`codegen_module`, `svc.compile`,
  `compile_for_profile`). Work's compiler marks a whole-module compile without an entry
  `ProviderRequired`. This existed before the merge (Sol-41 table). Either the compiler (Sol-45)
  or a harness move to `try_eval_source` resolves it, and the move only pays once C is fixed.
- **C, native-core entry refusal (Sol-45)**: `CompilationAdmission(NativePreflightRequired)`
  through the VM's own entry (`try_eval_source`). Seen in `string toupper/index/range/map/is`,
  `regsub/regexp`, `format`, `dict`, `package` vectors, `info`, `subst`, `clock`, `foreach`,
  `switch`, and execution-trace callbacks.
- **H, hand-built top-level LVT** (opcode_c_parity 16, opcode_catch_parity 7):
  `compiled local receiver is unavailable` / `compiled namespace local unavailable`. These
  hand-built `FunctionAsm`s use LVT slots at level 0 with no compiled-local layout, which a
  native-core VM refuses. It is a harness/VM-model mismatch, not a regression in the opcodes.
- **S, seeded compile service not consulted** (opcode_c_parity 8: TclOO/yield/coroName): the
  test's seeding `CompileService` is never asked for the procedure. Work's VM compiles procedures
  through the entry-bearing API that the seeding service does not override.
- **V, retired native object header**: a `Value` used after the interpreter that issued it is
  gone (F2). Seen in cmd_math_expr (3), c_error_paths `lsort -command nosuch`,
  cross_interp_alias and the lib.
- **P, profile flip on a native core** (command_mutation_deopt 5):
  `entered VM original execution context` after `set_dialect_profile` to another release on a
  running VM. The engine no longer follows the pin. That is by design of the decision, but these
  deopt tests assume it does.
- **O, OO recursion stack overflow** (cmd_oo_e2e `moderately_recursive_method_still_runs` and
  `deeply_recursive_method_errors_instead_of_crashing`): see F1. The whole cmd_oo_e2e binary
  aborts under libtest; under nextest only these two tests fail.
- **other**: semantic gaps, mostly the Sol-41 deferred items:
  - 8.4 numeric model / `StringUpdater` (release_number_semantics)
  - trace ordering (variable_trace_semantics)
  - tcl-vm-cli top-level coroutine settle, stdin errorInfo, `if {1} {break}`, 8.5+ nested
    direct-substitution levels
  - `{*}`/lassign under `--tcl-version` (bin)
  - worker preflight (channel_output)

## 3. Findings from the migration
- **F1 (crash)**: in a debug/test build, one TclOO method recursion level costs about 200 KB of
  native stack. Measured with gdb over the overflow:

  | frame | bytes per level |
  |---|---|
  | `Vm::tick_in_original_scope` (3117-line function) | ≈93 KB |
  | `cmd_oo::run_step_inner` | ≈48 KB |
  | `oo_run_method` | ≈18 KB |
  | `dispatch_selected_words` | ≈16 KB |

  - Depth 10 needs between 2 and 4 MiB; depth 20 (`OO_DISPATCH_DEPTH_LIMIT`) needs between 4 and
    8 MiB. A test thread has 2 MiB, so `OO_DISPATCH_DEPTH_LIMIT` (calibrated at about 43 KB per
    level) no longer protects it.
  - Fix options:
    - outline the INVOKE path out of `tick_in_original_scope`, or route method dispatch through
      the trampoline (the doc comment's own "correct fix");
    - or grow the stack (`stacker`, which is native-only).

  Not fixed here.
- **F2**: a `Value` read after its native-core `Vm` is dropped reads as `""` through
  `to_str()`, with no error. The result of a hand-built function, for example, after
  `let (_, c) = run_fresh(..)`. Elsewhere it is the typed `retired native object header`
  refusal. The silent empty string looks like a VM bug in the value-lifetime owner.
- **F3**: `test_support::native_vm_for_profile` cannot pin a vendor profile directly; callers
  pick a host core. Decide whether a helper should name the host core per vendor profile.
- **F4**: `native_mathfunc_bootstrap::check_constructor_roster` compared the core constructor
  against the distribution constructor (`Vm::new()`). That constructor is no longer reachable from
  integration tests, so its `previous` and `distribution` comparisons now compare core with core.
  They pass trivially and want a rewrite or removal by the test's owner.
- **F5**: tcl-engine-tclvm `guest_dictionary_options_refuse_an_unknown_native_issuer_without_shimmer`
  needs an interpreter with no native engine. Under the decision, no crate outside tcl-vm can
  build one; it was `Vm::new()` and is now `test_support::native_vm()`, and it fails. Its owner
  should move it into tcl-vm's unit tests, which keep `Vm::without_native_engine()`, or drop it.
- **F6**: `TclVmEngine` is now a core-bootstrap interpreter. It calls `set_host` after
  construction so the host seeds `tcl_library`/`auto_path`/`env`/`tcl_platform`, as the engine's
  `a_confined_engine_reads_no_host_environment` expects. It no longer has `argv`, `argv0`,
  `argc` or `tcl_interactive`.
- **F7**: the debugger hang (`launch_survives_deeply_nested_control_flow`, 400 nested `if`s) is
  in tcl-compiler lowering (`Lowerer::prove_possible_bodies` → `attach_evaluated_body` →
  `SourceCommandBindings::stamp_original_tokens` → `source_binding_from_original_input`,
  recursing for each level). It is not VM work: for Sol-45.

## 4. Deferred ports (task 4)
I spent most of the session on the migration and the re-measurement. This is all I did on the
ports.

### Item 4: errorInfo levels (`ErrorLogged::ThroughFrames`). VM half done
- `NativeErrorState.error_logged` (and the copy in `ParkedFlow`/saved flag states) is now the
  tri-state `interp::ErrorLogged { No, Yes, ThroughFrames }`, as upstream fcb7927f02 has it.
  - Frame-text appenders and `clear_error_logged` use `past_frame()`, which keeps
    `ThroughFrames`.
  - Procedure/lambda frames, `seed_error_info_frame`, trace frames, eval resets and the
    flow/unpark paths reset to `No`.
  - Saved and restored flags carry the full state.
- New `Vm::settle_expression_substitution_error`, called when a command substitution inside a
  runtime-evaluated expression fails. Call sites: `fold_expression_result` (the trampolined
  `expr`/`if $c` evaluator) and `ExprEval::command{,_bytes}`.
  - From 8.5 it clears `ERR_ALREADY_LOGGED` (the end of C's `Tcl_EvalObjEx`), so the enclosing
    command logs.
  - In 8.4 it keeps an already-logged error logged through frames, up to a procedure boundary.
- Measured against tclsh 8.4.20/8.5.19/8.6.16/9.0 on new vectors:
  `set w [list a [expr $e]]`, `proc q {set e ..; set w [list a [expr $e]]}`, the same inside
  `eval`, and `if $c {}` with `e`/`c` holding `[p]...`.
  - 8.4 matches C on all four.
  - 8.5+ now logs C's `invoked from within "expr $e"` level, which it lacked before.
  - What is still missing is the outer direct-statement levels (`list a [...]`, `set w [...]`).
    Those, and every *compiled* condition or braced expression (`if {[p]} {}`,
    `set w [expr {[p]}]`, `set y [string repeat [e] 2]`), need the compiler side of fcb7927f02:
    - `ErrorInfoCommand.enclosing` for condition commands and substitutions in directly
      evaluated statements (`terminator.rs::name_condition_command`);
    - `in_direct_statement` (`codegen/mod.rs`);
    - the no-fold rule (`expressions.rs`).

    These are rust/tcl-compiler changes, so they belong to Sol-45. I ported a VM-side PUSH-word
    hook and then removed it again: with no compiler metadata it is never reached, and its 8.4
    branch would contradict C on `set y [string repeat [e] 2]`, where tclsh 8.4.20 logs both
    enclosing levels.
- Test bars:
  - tcl-vm-cli script_errors and top_level_e2e are unchanged at 3/5 and 6/10; their remaining
    vectors are the compiled/direct ones above.
  - No tcl-vm regressions (see the after-check row below).
- `Frame.script_line_base` (errorLine for multi-command substitutions) is not ported.

### Child interpreter error objects (item 3 robustness)
`interp create` children now configure their error-object stack for their engine. Safe-child
hidden-command errors went from a `native error-context owner` refusal to C's
`invalid command name`.

### Not started
Items:
- 2: 8.4 running-bytecode release units
- 3 (rest):
  - `lsort -command` comparator `(procedure "cmp")` frames;
  - the `nosuch` comparator's retired header (V);
  - 8.5 stdin errorInfo / `return -code error` file trace;
  - `if {1} {break}`
- 5: switch/try/dict compiled frames
- 6: expr word substitution / 8.4 numeric model
- 7: lseq OPERATIONS, lsearch/lsort release, string shortcuts
- 8: literal binding
- 9: top-level coroutine settle, which still needs the top-level-role decision Sol-41 flagged
- 10: stdin history

Most of their test bars are still dominated by B/C/V (tables above).


## Patch scope
`sol-44.patch` = `git diff --` over:
- `Cargo.lock` (one line)
- `rust/tcl-vm`, `rust/tcl-vm-cli`, `rust/tcl-vm-wasm`, `rust/tcl-engine-tclvm`
- `rust/tcl-debugger`, `rust/tcl-fuzz`, `rust/tcl-irule-test`, `rust/tcl-spectcl`
- `rust/tcl-compiler/tests/wasm_codegen.rs`

The patch includes the new file `rust/tcl-vm/src/test_support.rs` via `git diff --no-index`.

## Gaps
- clippy: only `cargo clippy -p tcl-vm --lib --features test-support -- -D warnings` was run,
  filtered to the lines I changed (one finding, now fixed). The tcl-vm lib still carries
  pre-existing clippy debt elsewhere. Clippy was not run on test targets or on the other crates
  (disk was below 2.5 GB for long stretches).
- tcl-vm-wasm was not built for wasm32.
- Not every tcl-vm integration binary was re-run. Unlisted binaries, other than the
  rerun-on-change set, were changed only by the constructor swap; their results are unknown.
