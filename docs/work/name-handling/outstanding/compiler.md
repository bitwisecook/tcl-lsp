# Outstanding work: compiler agent

Area: Lowering, C6 fresh builders, iRules RuleLoader realm, itcl/snit, OO.

Last updated: 2026-10-22T17:30:32Z (compiler role resumed in the active checkout)

Maintained by the compiler agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full change notes: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/sol-49-NOTES.md`.

## Resume

**Resumed in `/workspace/naming-migration-active/tcl-lsp` at `d7224c706`.** Compiler-names has the first build lock for g4. C6 priorities are the opaque-switch conditional completion scan, deferred-write schema widening and reached cross-interpreter taint. N09 and authenticated ILX source-handle provenance follow C6. The perf realm/interprocedural patch remains unapplied pending its equality proof. No build has run for this role yet. Names released its first window at 15:51:25Z after one aggregate g4 main failure. Compiler-codegen now owns the unchanged N17 baseline plus its narrow functional fix chain. After its source hold ends, names runs a public normal-library g4 diagnosis, followed by this role's public C6 diagnosis. Stabilise the functional fixes before one compiler lib-test build for the original g4/ZZ/eight, three FP families and the LSP syntax minify issuer. Original regressions remain mandatory.

Evidence directory (in the repo): `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler/`.
- `scripts/`: historical batch scripts and receipts. Their executable deletion is incompatible with the current 60-minute retention rule; do not run them unchanged. The active guarded runners are `/tmp/compiler-resume-integration-run.sh` (temporary public diagnosis) and `/tmp/compiler-resume-run.sh` (original library filters), each under the coordinator-assigned build window and global lock. Historical results go to `$SOL49_SCRATCH/res/<tag>-<filter>.txt`.
  - `cycle2.sh <tag> <filters…>`: one command that builds the `tcl-compiler` lib tests, runs the `zz_sol49` probes with `--nocapture` (with `ZZ_SOL49_ANY=1 TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS=1`, written to `<tag>-probe.log`), runs each filter, then deletes the executable.
  - `cycle.sh`: the same, without the probes.
  - `cmp.sh <base.txt> <after.txt>`: prints pass-to-fail tests and the gain count.
- `sol49-results-z2-y3-d1.tar.gz`: per-test ok/FAILED lists. `z2`/`y3` are the pre-C6 baselines; `c1`/`d1`/`e1` are with C6.
- `e1-probe-summary.txt`: the probe findings. `e1-probe.log.gz` is the full log.
- `zz_sol49_probes.rs.txt`: copies of the two inline probe tests.

Partial C6 batch results, against z2:
- d1:
  - `diagnostics::fp` 484 ok / 97 failed: 3 pass-to-fail, 40 gains.
  - `interprocedural` 76 ok: 0 pass-to-fail.
  - `diagnostics::helpers` 13 ok.
  - `compilation_unit` 36 ok: 0 pass-to-fail.
  - `irules_event_checks` 77 ok: 0 pass-to-fail.
  - `bounds_checks` 56 ok.
- The coordinator pruned the d1 executable after `irules_event_checks`, so the later d1 filters ran no tests.
- e1 completed the build, the probes and `fp_rbs_15` (6 ok / 3 failed, unchanged), then stopped for the pause.
- Not yet measured with C6: `command_binding::`, `irules_checks::`, `diagnostics::tests`, `registry_invocation`, `signature_scan`, `taint`, `lowering`, `source_class`, `depth_guard`, `cfg_builder`, `deferred_writes`, and the integration tests `optimiser_coverage` and `spec_corpus` I230.

Probe findings (e1), to act on next:
- **`fp_rbs_15_while1`.** In the procedure CFG, the loop body block `B2` (holding the switch) only goes back to the header `B1`; nothing wires it to the exit block `B3`. So the switch's `break` never reaches `lower_opaque_switch`'s escape wiring: either the switch is not lowered as an opaque switch inside the `while 1` body, or `switch_escaping_jumps_with_classes` returns false. Check `cfg_lower.rs` along the path from the `while` body to the switch statement.
- **`fp_sh_21`.** `deferred_writes.any` is set in `deferred_writes/supplied.rs::original_registration` at three points:
  - `with_metadata_schema` / `selected_values` returns None (the instrumented line 148, most frequent);
  - no `original_materialized_footprint_for_module` (line 118);
  - `deferred_namespace(value.scope)` is None (line 154).

  Next step: log the command name at each of these points.
- **`fp_tnt_03`.** With an unset `$child`, the TN `interp eval $child [list puts $raw]` still emits T105. The nested `[list]` invocation is unreachable, so `taint.rs::list_wrapped_arg_command_is_literal` gets no proved wrapper invocation. With a created child (`set child [interp create]`), the TP `interp eval $child [list $raw]` emits nothing at all, which is a real FN.
- **hover_residual.** At compiler level the schema is NOT withdrawn. For `regexp {plainword} $line` (with `$line` undefined), `source_registry_words` gives words and `with_source_schema(...).authored_source_pattern_arguments()` gives 1 pattern; `lsearch -exact $list $needle` also gives words. The withdrawal must be on the LSP side (`tcl-lsp-core` `hover.rs` / `original_invocation.rs` walk, or the registry snapshot-key check). That is outside my files.

Historical e2 command, superseded by the guarded runners above. `cycle2.sh` deletes the fresh executable and must not run unchanged. Its pending original filter scope remains:
`SOL49_SCRATCH=<scratch> THREADS=3 docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler/scripts/cycle2.sh e2 fp_rbs_15 fp_sh_21 fp_tnt_03 command_binding:: irules_checks:: analyser::diagnostics::tests registry_invocation signature_scan:: taint:: lowering:: source_class::tests depth_guard::tests::cap_deep depth_guard::tests::deep_namespace_eval_lowering cfg_builder:: deferred_writes`

Temporary code to remove before hand-back (all of it compiles):
- `zz_sol49_dict_for_probe` in `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs`
- `zz_sol49_hover_schema_probe` in `rust/tcl-compiler/src/registry_invocation/source_structure.rs`
- `zz_sol49_any` in `rust/tcl-compiler/src/deferred_writes/supplied.rs` and `rust/tcl-compiler/src/deferred_writes.rs`: restore every `zz_sol49_any(line!())` to `true` (and the two `&& zz_sol49_any(line!())` operands), then delete the function.

Files I have touched since e34d8a427e (details in `sol-49-NOTES.md`):
- `ir.rs`
- `lowering/mod.rs`
- `depth_guard.rs`
- `source_transition_advice/source_class.rs`, `source_transition_advice.rs`, `source_procedure.rs`
- `command_binding.rs`: re-export and C6 hunks
- `realm.rs`, `realm/source_transition_advice.rs`
- `registry_invocation/source_structure.rs`
- `analyser/original_members.rs`, `analyser/handlers.rs`, `analyser/irules_event_checks.rs`, `analyser/bounds_checks.rs`
- `cfg_builder/cfg_lower.rs`
- `deferred_writes/supplied.rs`, `deferred_writes.rs`
- `compilation_unit.rs`
- the C6 companion files
- `fp/inj.rs`, `fp/sh.rs`
- `docs/design/contracts/naming-consumer-matrix.md`

## In progress

- **R1 peer review (read-only during the full Rust/test source hold).** The initial callback-registration draft lacked a deferred-script guard: `after`/`fileevent` are Structural, and concatenated timer scripts intentionally have no single-script ordinal. The positive-only deferred ordinal API cannot prove no retained hook. Off-tree design `/tmp/compiler-r1-callback-registration-proposal.md` separates selected registry registration from callback capability, including getter/removal forms and unresolved callback options. Five C Tcl registration inventories all exited 0 with empty stderr; source/executable hashes and exact observations are preserved in `../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/r1-callback-registration-native-oracles.json`, with review status beside it (`r1-callback-registration-review.md`). Minimal frozen source-retention fact and 3 registry tests are drafted in `/tmp/compiler-r1-retention-facts.patch` (rustfmt parse/apply-check only); no Cargo validation or canonical Rust/test changes. Names reviewed the frozen typed fact and three tests, and its revised guard consumes Absent/MayRetain/Unproven while keeping independent precise CP/HOST/trace/namespace guards. The registry fact plus three tests were applied after the coordinator's explicit post-rebase GO on `de4455665fedeae9eba3c011585871c0121498d7`; rustfmt file check and diff whitespace check pass. The file SHA256 is `cc877f233de0030a0ee428ad3866d497832c0b8e86983b558202fde124055785`. Names has applied its consumer and holds the first public normal-library g4 slot (started 17:09:43Z); this role's C6 diagnosis follows its explicit release. The public R1 aggregate passed 1/1 with all 54 subcases (14 positive and 40 negative), including nonempty original body-lookup checks; names reports zero warnings/errors and zero changes across 6510 source hashes. Names released the lock at 17:27:36Z. No Cargo/test execution or getter/removal precision claim is made. The exact registration descriptor design remains a separate follow-up.

- **Build status / source hold:** initial test compilation found nine missing tracked experiment includes and executed zero tests; all 166 inputs were then restored. The retry built the normal library and test crate, and names' g4 main finished **0/1 aggregate tests**, with ten positive command-world subcase mismatches (168.78s). There is no eight-assertion attribution result. The following ZZ rebuild was retired after the coordinator identified a transient lexer `parse_cut.rs` edit/restore by coordination support; no fingerprint override is authorised or applied. Names released the lock at 15:51:25Z. This role has run no Rust test or gate and holds all Compiler source mutations since 14:44Z through codegen's unchanged N17 baseline and narrow functional fix. The approved public integration diagnosis probe remains `/tmp/zz_compiler_resume_probe.rs`, with exact original C6 inputs/expectations, reached controls, stage timings and N09 caller/namespace-path edge controls. `/tmp/compiler-resume-integration-run.sh` retains `.rs.txt`, HEAD/current Rust diff, source/command/exit hashes and capped logs in unique receipt directories; the root-authorised `original_c6_inputs` run started at actual 17:28:00Z, with receipts under `/tmp/compiler-resume-integration/20261022T172800513712783Z/`. It captures full before/after Rust/Cargo source manifests and retains the new executable plus hash/age. The diagnosis failed 0/1 in 82.12s (cargo 101/logger 0), with five of eight row mismatches: all three original FP issues persist and both reached-child tainted-head controls miss T105. Original tainted-head TP, S101 absence and reached literal-head TN pass. All 5396 Rust/Cargo hashes are unchanged; HEAD advanced a1dd to d7ff via the coordinator's announced docs-only publication. The temporary source is removed and executable SHA `0e09040aa4dc74d82ce3635f0e7a00675d0835ffcfb0ea16ba1e839ef369c5cb` retained. Receipts are preserved in `../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/c6-public-20261022T172800Z/`. Root authorises precise off-tree functional correction drafts before the coordinated source batch. This does not substitute the original regression filters.
- **ASCII descriptor peer review:** read-only review of `/tmp/ccg-string-is-ascii.patch` found no static registry authority blocker for exact non-strict C8.6+ ASCII. Existing public/private worker identity prerequisites remain independent; strict/failindex/dynamic class/expansions stay Unknown. Pinned modern C sources and finite 20-control observations agree. Receipt `/tmp/compiler-string-is-ascii-registry-review.json`; no apply/Cargo or semantic acceptance. Dedicated selector/prerequisite negatives and same-body positive remain pending.
- **Registry assertion-style lint follow-up:** the coordinator routed 40 new registry integration assertion-style findings from the external consumer inventory. The coordinator delegated the mechanical off-tree draft to coordination_checks; this role retains review/integration ownership. Preserve every assertion condition and add no allowances. No compiler-owned duplicate patch or acceptance is claimed.
- **Release lint work queued:** `/tmp/naming-clippy-compiler.json` assigns 565 old-snapshot diagnostics across 132 paths; names additionally transferred eight in `source_transition_advice/body_effects.rs` and four C6 root binding diagnostics (`analyse_deferred_entries`, `walk_deferred_entry_with`). No allowances or assertion changes. `/tmp/naming-clippy-compiler-current-map.json` maps all 577 assigned old-snapshot diagnostics to unchanged current lines across 134 SHA256-tagged files. `/tmp/compiler-clippy-pass1/pass1.patch` consolidates 160 uncompiled mechanical candidates across 50 paths (80 docs/borrows/copied iterators, 46 type-inspected Copy-clone replacements and 34 method references); nothing is applied. Earlier drafts remain under `/tmp/compiler-clippy-offtree/`. `/tmp/compiler-invocation-metadata-selection.patch` supplies the coordinated crate-private Standalone/Retained enum with an outer refusal and migrates the retained-owner adapter; names and codegen have its exact API. Separate off-tree C6 frame-selection, hierarchy and W220 message helper extractions plus a private source-member Absent/Present enum parse with rustfmt but are uncompiled. Long functions and other structural lints remain outstanding. `/tmp/compiler-clippy-pass2/pass2.patch` adds 48 uncompiled mapped mechanical candidates across 25 paths. `/tmp/compiler-metadata-selection-consumers/compatibility.patch` adapts 16 fallback/SSA/static-loop paths to the shared accepted Selection without changing existing helper refusal meaning; its pre-existing private Option<Option> helper signatures still need their own lint-stage conversion. Three private boolean-storage groupings (`ir_helpers`, SSA source-use snapshots, iRules drop/HTTP flow) and seven type-complexity aliases are drafted under `/tmp/compiler-boolean-storage-lint/` and `/tmp/compiler-type-alias-lint/`. These all parse/apply-check only. Names assembled ten off-tree patches across 49 copied paths successfully; receipt `/tmp/compiler-names-g4/coordinated-batch-applycheck.json`. The two exact-body def-use helper extractions in `/tmp/compiler-def-use-lint.patch` also parse/apply-check only. Source-lambda/frame mutable-key lints must retain full sealed Eq owners; hash buckets may index full receipts as values, never serve as identities. No issuer equality changes or lint batches have been applied.

- **Resumed C6 candidates (unmeasured).** `lowering/mod.rs` now falls back to the authenticated original Native source structure reader; that reader retains its original SourceTransitions issuer and adds registry-described Switch/case bodies. Hosted and Native readers share the same source body layout checks. The opaque switch escape scan also uses its existing conditional source completion owner, including the dead-code cutoff. The e1 evidence showed the switch was a generic Call, so both structure and completion are needed. Temporary exact-command deferred/taint traces and expanded probes await the first build window. A source-schema-only quotation suppression candidate was withdrawn before measurement because it did not establish closure of unknown execution alternatives.
- **Native controls:** `handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/c6-native-oracles.json` records Tcl 8.4/8.5/8.6/9.0/9.1. All five reach the post-switch undefined read, execute a tainted constructed-list head, and keep a constructed-list argument quoted. These are semantic controls, not Rust acceptance. `c6-original-interp-oracles.json` additionally runs both exact original `gets stdin` fixtures as files on all five C releases: each exits 1 at the unset `child` read before the list substitution; any fixture initialisation correction must cite this evidence and retain positive/negative expectations.
- **New routed work after C6:** N09's missing `::n::c` summary and authenticated ILX constructor→setter→later source-variable relation. ILX keeps runtime `target` meaning and existing negative controls; no consumer-local handle walker. `/tmp/compiler-ilx-source-owner-api.md` records the unimplemented separate remote result/setter/scalar-read owner and required invalidations. The existing registry selected-source variable write projection can supply withdrawal names without reconstructing lookup; unknown writes, observer changes, foreign frames and operand barriers must withdraw the relation. Jim overriding-pack `proc` metadata may lack the native procedure-definition descriptor; VM execution confirmation is pending.

- **C6 zero-regression pass.** Landed in the tree: the companion and the WIP hunks, plus:
  - the `bounds_checks` W241 switch-clause fix;
  - the `cfg_lower` opaque-switch loop-escape ordering;
  - the `supplied.rs` lowered-child skip;
  - the `fp_inj` and `compilation_unit` corrections;
  - the CRLF lowering fix.

  Three `diagnostics::fp` pass-to-fail remain (see the probe findings above).
- **Eight-test attribution remains pending after the retired ZZ run.** Static joint review separates six uncalled-procedure assertions from the explicit real-Native-entry conditional-body preparation case; the latter must distinguish compiler visits from runtime reachability, retaining its conditional runtime refusals. The import allocation case stays with names' R1 gate/closed-world fix first. Native compiler disassembly controls are retained under `/tmp/compiler-names-g4/`; no assertions have changed.
- **Classifying compiler-names' 8 `command_binding::` pass-to-fail** (`compiler-names/g1-command_binding-pass2fail.txt`). Started by reading the tests; not run yet. First reading:
  - `expression_preparation` ×2, `lifecycle_advice`, `normal_result`, `declaration_layout`: they assert that an UNCALLED `proc` body has no proved execution target. The user decision of 2026-10-21T08:20Z ("world after load") makes those commands resolve, so these are likely (a), each needing a per-assertion correction that cites the decision.
  - `conditional_expression` "Reached": comes from `walk_deferred_entry_with`, which sets `declaration_preview_depth` to 0 for an after-load entry, so `runtime_reachability` becomes Reached (`command_binding.rs` around line 3052). With preview depth 0, the uncalled body also records actual compiler visits and failures and marks called implementations. That needs a decision: keep Reached as the modelled after-load call, or add a separate reachability. Not settled.
  - `original_body_namespace`: comes from the C6 `record_conditional_in_frame_script` hunk. A conditional in-frame `if` body now reports the namespace it would run in (`::`); likely (a), with the correction `Some(expected)`.
  - `source_command_world` import: `original_completed_command_world()` is None. That is compiler-names' R1 area, but it may be C6's fresh entry (`rename auto_import`); needs a run.

## Outstanding (in priority order)

- **N17 derived SCCP admission (queued after R1/C6 diagnosis).** The completed combined public diagnostic reports direct Evaluate success (expr42/catxy, default and EMPTY contexts) while all eight CU rows retain `::p`/trusted heads but only one known SSA v1 Overdefined, no other versions, no SCCP routes, and no host errors/crashes. Attribute this pre-route admission separately from CCG's explicit-body native preflight-provider refusal. CCG owns that provider seam; this role owns derived SCCP admission/route attribution after the functional diagnosis. No runtime parity conclusion or canonical fix is claimed. Current source hold still blocks edits/builds until the coordinator's post-rebase GO.

1. Finish C6. After the running build, run the full tcl-compiler filter batch:
   - `interprocedural`, `diagnostics::fp`/`tests`/`helpers`
   - `compilation_unit`, `irules_*`, `registry_invocation`
   - `signature_scan`, `taint`, `lowering`, `command_binding`, `depth_guard`, `source_class`

   Then the integration binaries: `optimiser_coverage`, and `tcl-spectcl` `spec_corpus` `shared_unit_matches_legacy_for_representative_profiles` for I230. Zero pass-to-fail, or a documented correction.
2. `fp_tnt_03_interp_eval_list_literal_head_no_t105` (C6 blocker).
   - Under the fresh interpreter `$child` is unset, so the nested `[list …]` invocation is never reached and its suppression doesn't apply.
   - Creating the child in the fixture made the TP sibling (`…_tainted_head_fires_t105`) lose T105. That is a real FN for a created interpreter, so I reverted the fixture change. Needs a fix in the T105 sink or suppression.
3. Hand-back gates for my files:
   - `cargo fmt --check`
   - Strict compiler Clippy for the release lib/bin/test scope, zero warnings across the assigned inventory and new code (the `derived_hash_with_manual_eq` on `ir::Script` is already fixed); no new lint allowances
   - `cargo check --workspace --all-targets`
   - remove the temporary `zz_sol49_dict_for_probe` test
4. IRULE4001/6001 in hosted procedure and event bodies (`source_graph::ordinary_rule_init_reporting_name_*`, `irule4001_fires_for_static_write_outside_rule_init`, `irule6001_*`). The body's `set ::static::x` has no original variable operands, so `NormalTransferInvocation::mutation_places` returns `unknown_top`. These were already failing before my iRules work.
5. Review findings, pending triage routing:
   - The triage report `docs/work/name-handling/reviews/naming-migration-review-20261022-triage.md` (12:43Z) lists N08 and §3.3 `method_barrier` as FIXED.
   - Still open, not yet routed to me:
     - N09: external retry at `128e1bb4` reports constructor/callgraph correct (one observational test, 64.04 s); widened taint `resolved_callees(::n::c)` retains `::p`, `::lib::p`, `::n::c` as a conservative may-set. Static workspace search finds only one production consumer, the taint fixpoint caller requeue map; no definite target is selected there and no direct LSP/transform consumer of that function was found. Keep this scoped to dependency precision until a wrong enabled consumer is demonstrated; do not expand ahead of C6/ILX.
     - N10: a class selected through `namespace path` still gives no hover, type hierarchy or definition.

## Blocked (on whom, and what is needed)

- **itcl contract-D extension.** Affects 4 tcl-lsp-core `name_resolution` `itcl_class_proc_dispatch` tests: `tp_every_dispatch_spelling…`, `tp_references_from_the_call_site…`, `fn_to_tp_the_class_resolver_pre_empts…`, `fp_a_shadowing_proc_wins…`.
  - (a)/(c) In-namespace class declarations and calls after them: the advice graph treats every `namespace eval` body as a possible branch (`source_transition_advice/body_effects.rs::retain_possible_effects` only records possible occupants). That is compiler-names' file, and the change is design-level: install a definitely entered body's publications in the parent graph. Needs an owner decision. TclOO ctor calls after a namespace-eval class have the same limit.
  - (b) Calls inside itcl member bodies (`[make]` in `proc other`, `[Factory::make]` in `method inst`) need a new advice walk of itcl member bodies under the class-namespace resolver.
- **LSP `name_resolution` tests outside my items:** `namespace_scoped_class_dispatch::{fn_to_tp_an_explicit_namespace_import_is_followed, tn_object_command_dispatch_is_scoped_to_its_creation_namespace}`, `pkgindex_dir::fp_other_undefined_var_in_pkgindex_still_fires`.
- **Compiler-names R1 completed-world change.** 10 `command_binding::` tests assert `original_completed_command_world().is_none()` and now fail. Routed to compiler-names.

## Done since the last update (with evidence: test counts, commit or snapshot)

- **Compiled-word miscompile.** `string length "{}$z"` now gives 3. Test `native_operand_words_in_a_substitution_keep_their_variables_live`, tcl8.4–9.1.
- **Stack overflow at the structured depth cap.**
  - Segmented-stack `Script` Clone/PartialEq, manual Hash.
  - Take/borrow instead of clone in `extract_oo_methods_pass` / `populate_trace_facts`.
  - `lower_command` split: about 29 → 18 KiB per level.
  - Tests: `depth_guard::{cap_deep_structured_if_ir_clones_and_compares_within_the_budget, deep_namespace_eval_lowering_fits_the_budget}`.
- **Contract D.**
  - snit typemethod is now a ClassObject member.
  - New `source_class_proc_call_at` / `OriginalSourceClassProcCall` for itcl `Class::proc`, with shadowing.
  - The itcl two-word form stays construction.
  - LSP: `tp_snit_typemethod_bare_dispatch_is_a_reference`, `fn_to_tp_itcl_class_proc_colon_dispatch_is_a_reference` and `fp_itcl_bare_two_word…` pass.
- **iRules RuleLoader realm.**
  - `proc` is an original declaration with an original lookup (`handlers.rs` declaration policy, `realm.rs` hosted document realm).
  - The IRULE5001 `static::` gate is spelling-aware.
  - tcl-lsp-core `name_resolution` is 70/77; `irules_switch_glob_body_is_followed` gives `[0]`.
- **C6 applied:** the companion plus `sol-49-c6-wip.patch`: after-load, condition slot, conditional in-frame body namespace. Measured without the realm change:
  - `interprocedural` 55 → 76 (0 regressions)
  - `diagnostics::helpers` 5 → 13
  - `diagnostics::fp` 447 → 479
  - remaining blockers are listed above
- **Pre-C6 batch:** 0 pass-to-fail from my changes across `irules_event_checks`, `irules_checks`, `diagnostics::tests`/`fp`, `registry_invocation`, `signature_scan`, `taint`, `lowering`.

## Coordinator note (2026-10-22T13:30Z)

- The "reached" question is decided (`user-decisions-20261022T1317Z.json`): analysis models the code once loaded and running, so for after-load entries the preview-depth-0 walk that records actual compiler visits stays the modelled call (no separate reachability value). Apply narrow test corrections to the 8 `command_binding::` tests where they contradict this, citing that decision and the 2026-10-21T08:20Z decision; ask the user about any case that does not fit.
- hover_residual: the schema is lost on the tcl-lsp-core side (`hover.rs` / `original_invocation.rs`), so it belongs to the lsp role (noted in outstanding/lsp.md).
- Review N09/N10 residuals (no call-graph summary for `::n::c`; a class selected through `namespace path` gets no hover, type hierarchy or definition) are routed to compiler.
