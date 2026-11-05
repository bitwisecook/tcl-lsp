# LSP release full-suite sweep

Updated 5 November 2026. Branch: `work`. The LSP is not release-ready.

Scope: LSP, compiler and SpecTcl. VM defects that do not affect SpecTcl, BIG-IP
(iRules, bigip-report-gen, f5-*, bpf-tcl) and tooling are recorded but deferred.

## How the suite was run

`make test-rust` cannot run in one pass inside the session's 11 GB disk
allowance (the 380 integration binaries alone exceed it) or 15 GB memory cgroup.
The run therefore used `cargo test --workspace --all-features --no-run` per
batch of 25 test targets, so feature unification never changes, then ran each
executable from its package directory with an 8 GB address-space cap and a
30-minute wall-clock cap, four binaries at a time. Profiles were
`opt-level=1`, `debug=0` (debug assertions and overflow checks unchanged).

Baseline source `acaa85890`: 129 of ~430 test binaries (integration targets
`a*`–`cr*`) ran before fixing started: 2160 passed, 695 failed, 80 failing
binaries. The remaining batches, unit (`--lib`) and doc tests run against each
pushed fix commit; their numbers are added below as they complete.

## Failure clusters (ROI order)

| # | Cluster | Scope | Evidence | Status |
| --- | --- | --- | --- | --- |
| 0 | `package require` of anything makes every later command an unproved binding, so W210/W214, call-graph edges and compiler folds vanish below a file's prelude. Fixed for the interpreter's own packages (fix 2). For other packages the decision is to trust a loader limited to its declared surface when a loaded SpecTcl pack or the shipped registry declares the package (fix 3, in progress). | LSP, SpecTcl, product | `tcl diag`/`tcl callgraph` with and without `package require Tcl 8.6`; `explore` reports `unresolved-unproved-binding` | Partly fixed |
| 0b | LSP server `e2e` suite: 1020 passed, 412 failed, then aborted (panic in a destructor during `semantic_tokens_reference_client::large_file_first_semantic_tokens_response_is_prompt`; one `memory allocation … failed`). Failures span diagnostics (62), iRules (33), rename (25), definition (25), rename safety (24), code actions (24), references (19), recovery (18). | LSP, product | `tcl-lsp-server` `tests/e2e.rs` at fix 2 | Triage next |
| 1 | Test fixtures lower with a profile-less `CommandRegistry::build_default()` or a lexer config that does not match the registry's profile. The name-resolution contract makes both abstain, so commands stay generic `Call`s and procedures are never materialised (`fixture body "::p" absent`). | Test harness (hundreds of `tcl-compiler`, `tcl-lsp-core`, `tcl-lsp-db` failures) | `cfg.rs`, `codegen*.rs`, `checks.rs`, `core_analyses.rs`, `dataflow.rs`, `compiler_residual.rs`, `alias_scoping.rs`, `caller_frame_effects.rs` | Partly fixed (fix 1); remaining files need the same migration |
| 2 | SpecTcl pack `const_fold` hooks never dispatch in the optimiser; CLI `opt` with `package require` does not fold `::tcl_lsp_fixture::strlen abcde`. Builtin folds work. | SpecTcl, product | `tcl-cli` `cli_publishes_project_pack_hooks`; `tcl-spec-hooks` `const_fold_e2e`; trace shows no `dispatch-entry` | Open |
| 3 | Pack-defined `try` respelling lowers to `Try` but its bodies lose their definitions under dialect-aware lowering. | SpecTcl, product | `cfg.rs::a_try_handler_walk_reads_timing_not_keywords` (exposed by fix 1) | Open |
| 4 | Call graph drops command-prefix callback edges (`lsort -command`, `chan copy -command`, `hook bind`, `bibtex -recordcommand`). | LSP call hierarchy, product | `tcl-lsp-core` `command_prefix_integration.rs` (27 failures); `tcl callgraph` omits `main → cmp` | Open |
| 5 | LSP code actions return nothing: invert comparison, De Morgan, inline single-command proc, switch→dict, T101/T103 fixes. | LSP, needs product check | `tcl-lsp-core` `code_actions_depth.rs` (12) | Open |
| 6 | Cross-file metaclass chains converge on the first link only. | LSP workspace DB | `tcl-lsp-db` `class_factory_fixpoint.rs` (8) | Open |
| 7 | Memory blow-up: `tcl-compiler` `analyser` and `compiler_analysis_residual` test processes exceed 13.8 GB and are OOM-killed. | Robustness | kernel OOM log; test `class_factories::deeply_nested_collection_loops_do_not_blow_the_static_walk_stack` was running | Open |
| 8 | Every VM compile re-assembles a `ContextRegistry` (`Lowerer::with_dialect` → `ContextRegistry::assemble` → version-set parsing). `codegen_stamps::every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site` ran over 60 CPU-minutes. | Performance, affects SpecTcl hook execution | gdb stacks | Open |
| 9 | Optimiser declines: wide-integer and denormal double folds, profile second pass, regenerated `samples/optimiser` outputs. | Compiler | `tcl-cli` `cli.rs` `opt_*`, `samples_optimiser_profiles_are_regenerated` | Open |
| 10 | Jim programs report W302/W307; CR-terminated documents diff and minimise differently from their LF twins. | LSP diagnostics | `tcl-cli` `cli.rs` | Open |
| 11 | Spec-pack commands without `package require` are now unproved bindings: W120 is reported but arity/E002 and `resolved` are not. | Intentional fail-closed; tests need updating | `tcl-cli` `cli.rs` pack tests | Open (test update) |
| 12 | Bytecode codegen no longer inlines `if`/`for`/`foreach`/`switch` (`tcl dis` shows `invokeStk1`); golden corpus diverges. | VM performance (SpecTcl hooks slower, not wrong) | `codegen_golden.rs`, `codegen.rs`, `tcl dis` | Deferred |
| 13 | VM e2e harnesses lack a native compiler preflight provider (`NativePreflightRequired`). | VM | `tcl-vm`, `tcl-vm-cli`, `tcl-engine-tclvm` | Deferred unless `make test-spectcl-compat` fails on it |
| 14 | BIG-IP, bigip-report-gen, bpf-tcl, f5-*; dialect-blind iRules lowering in `irules_checks.rs`. | Out of scope | | Deferred |

## Fix log

| Fix | Commit | Change | Measured effect |
| --- | --- | --- | --- |
| 1 | `453e4ca35` (pushed as `285869e8d`) | `lower_to_ir` lexes and resolves under the registry's own profile; `codegen.rs` fixtures use a tcl8.6-projected registry | Across the 43 binaries that call `lower_to_ir`: 85 tests newly pass, 1 newly fails (cluster 3, exposed). `codegen` 106→37, `cfg` 14→7, `alias_scoping` 7→2, `compiler_residual` 23→20. `dialect_threading` (no `lower_to_ir` use) has 5 iRules failures, cluster 14 |

| 2 | `db42ac365` | Seed the packages a fresh interpreter provides (`TclVersion::core_provided_packages`: `Tcl`, `tcl`, `TclOO`, `tcl::oo`) as present, so requiring one is a version check, not an unknown loader | `package require Tcl 8.6` keeps W210/W214 and call-graph edges; 3 new `checks.rs` regression tests pass on 8.4–9.1; no new failures in `checks`, `cli`, `taint`, `optimiser`, `builtins_e2e`, `cmd_oo_e2e`, `pkg_verbs` against baseline |
| 3 | `7f1d9ccce` | A `package require` of a package the registry declares (loaded `SpecTcl` pack, or shipped stdlib/tcllib/Tk spec) selects a trusted loader of exactly the declared surface, resolved lazily from the baseline's registry; undeclared packages stay unknown loaders. Runtime, inherited and iRules entries never assert one. | `package require http`, `http 2.9`, `struct::set` and the tiny-project pack keep W210/W214 and call-graph edges; `no_such_package` stays fail-closed. Remaining: the package's own commands (`http::geturl`, the pack's `strlen`) are still `unresolved-unproved-binding` at call sites (`unknown=true`, no targets), so pack `const_fold` hooks still do not apply. |
| 3a | this commit | Naming review of fixes 2 and 3: package names compare as `NativePackageNameKey` under the binding world's selected `NamePolicyProtocol`; the declared surface comes from registry descriptor keys (`rooted_binding_label`); interpreter-provided package keys are projected through the same package-key owner. | Behaviour unchanged on the probes (8.4/8.6/9.0 `package require Tcl`, `http`, `struct::set 2`, pack, `nosuchpkg`). Pre-existing gap recorded: `apply_package_transition` still keys `state.packages` and trusted loaders by raw literal on both `Provide` and `Require`; converting one side alone would split NUL-bearing names, so both move together in a follow-up. Verified: `checks` (5 new tests pass), `cli`, `taint`, `optimiser`, `release_optimiser_safety`, `signature_scan`, `lsp_external_*`, `pkg_verbs`, `builtins_e2e`, `cmd_oo_e2e` show no new failures against fix 2. |

## Not yet run

Remaining integration batches, `--lib`, `--bins`/`--examples`, doc tests,
`make test-ext`, `runtime-rust-test`, `zed-query-check`,
`make test-spectcl-compat`, `make rust-check`, `make prep-pr`.
