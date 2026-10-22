# LSP agent (after Luna-41) — measurements, fixes, routed causes

Owned paths: `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server` (not the BIG-IP
modules). I committed nothing; the coordinator's WIP snapshots carry my edits (last seen in
`56893780b9` and later). Run conditions: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0`, `TCL_LSP_REQUIRE_TCLSH=9.0,8.6`,
`TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`, `--test-threads=2`, each test executable deleted
after its run. Numbers are "HEAD + shared working tree at that moment"; compiler agents landed
changes between runs. Temporary probes (`tests/zz_lsp_agent_probe.rs`) were deleted after use.

## 1. Integration targets (tcl-lsp-core) — before / after

origin/rust passes all of these (validation row 20: 239/239 incl. name_resolution 75/75 on
`fcb7927f02`).

| target | Luna-41 hand-back | first run here (nr0/h1) | latest run | remaining failures (owner) |
|---|---|---|---|---|
| name_resolution | 69/77 | 68/77 | **69/77** (p5) | itcl ×4 (compiler: in-namespace class join + calls inside class-member bodies), `fn_to_tp_an_explicit_namespace_import_is_followed` (C6, compiler-names), `tn_object_command_dispatch_is_scoped_to_its_creation_namespace` (compiler-names, §3a), `irules_switch_glob_body_is_followed` (iRules `proc` invocation, §3d), `pkgindex_dir::fp_other_undefined_var_in_pkgindex_still_fires` (C9 W210) |
| hover_completion | 50/56 | 52/56 | **55/56** (p5) → 56/56 expected with the past-end-of-line fix (pending re-run) | — |
| hover_residual | 27/36 | 28/36 | **33/36** (p4) | option word / 2× regexp pattern with an undefined `$var` operand (compiler, §3c) |
| references_residual | 16/23 | 16/23 | **21/23** (p5) | `document_highlights_dedup_keeps_write_over_read_on_collision` (synthetic `AnalysisResult::default()` + injected reporting map — the original-context contract refuses it; needs a decision), `references_external_obj_method_inside_proc_body_subst` (tclsh contradicts the fixture: `$d` is unset inside `proc run`; needs a decision) |
| references_rename | 70/98 | — | **71/98** (p4) | ~15 refuse `unknown-completed-command-world` (`puts [greet]`, compiler R1 not effective on this tree, §3b), class/object-variable renames refuse (§3b), itcl rename ×2 (in-namespace join), others listed in `scratchpad/lsp/out-p4-references_rename.log` |
| force_import_shadow_consumers | 2/10 | 2/10 | **6/10** (p5, ABSTAIN decision) | 4 not-shadowed halves that never passed on work: inline-proc / code-action offer no local `{args}` body, prepare_rename None, rename refused `unknown-completed-command-world` |
| call_hierarchy | (7/7 row 20a) | — | **7/7** | — |
| inlay_hints | (9/9 row 20) | — | **9/9** | — |

(Draft — sections 2–6 follow at hand-back.)
