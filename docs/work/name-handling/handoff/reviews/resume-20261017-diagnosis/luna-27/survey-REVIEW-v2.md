# Luna-27 delta review: sol-23 `survey-logical-control-v2.patch` against v1

**Verdict: ADMIT-TO-BUILD.**

This review is static only: no cargo, no repository writes, no worktree. Apply checks used a temporary `GIT_INDEX_FILE`. The tclsh oracle was re-run from my own copy of the probe.

## Identity and application

- **Patch:** sha256 `c975039dc9806e7d7998419a80c49f0c349a70b51d8526ddabf0ad8cdadf6e63`, 1048 lines.
- **Apply order:** on HEAD `106babc99`, Sol-20 `reachability-test-helpers-removal.patch` applied first with `--cached`, including its `manifest.json` hunk. v2 then applied cleanly with `--cached` (16 files in total).
- **Formatting:** `rustfmt --edition 2024 --check` is clean on every touched crate root and file. I checked against a full export of `tcl-compiler/src`, `tcl-registry/src` and `package_resolver`.
- **Hygiene:** the patch adds no `#[allow]` and no `eprintln!`.

## 1. The depth cap now bounds this chain: yes

- **The v1 defect is confirmed statically.** v1 passed the caller's own context to `walk_conditional`. Below that, `walk_body_operand` → `boxed_body_operand_context` → `compiled_invocation::body_context` (`..context`) keeps `depth` unchanged. As a result, `walk_source_image`'s guard, `MAX_SOURCE_NEST_DEPTH.exceeded(depth)` at `command_binding.rs` ~8569, never fired for survey bodies. This explains the 200-level overflow Sol-24 measured.
- **v2 fixes it.** It passes `*source_body_context_boxed(context, context.namespace, context.frame)`, which sets `depth: context.depth + 1` and `NestedScript`. That is the same constructor the Native `walk_selected_native_body` uses before `walk_body_flow`.
- **How the chain is bounded.** Each survey `if` level adds exactly 1 to `depth` before the guarded `walk_source_image`, so descent stops at the shared cap of 64. Bracket substitution nesting stays under its own `MAX_BRACKET_TEXT_DEPTH`.
- **Test coverage of the cap.** The new `package_survey_conditional_bodies_fit_the_shared_stack_budget` asserts both properties on a thread of `MIN_SOURCE_WALK_STACK − SOURCE_WALK_STACK_RESERVE`:
  - all `cap − 2` levels are entered;
  - at `cap × 4`, no more than `cap` levels are entered.

  It counts levels through the public `retained_body` lineage, and the capture runs inside the budget thread.

## 2. Frame size per level: plausibly within the envelope (unmeasured)

The recursive chain per survey level is:

`walk_source_image` → `walk_source_command` → `walk_evaluated_source_command` → `walk_selected_source_command` → `walk_logical_survey_command` → `walk_conditional` → `walk_body_operand` → `walk_source_image`

- **Shared frames.** All but one of those frames are shared with the Native conditional chain, which `depth_guard.rs` measured at 17,952 B per level.
- **Frames dropped compared with Native.** The survey chain does not include `walk_source_target`, the selected-native-body frames or `walk_body_flow`.
- **`prepare_selected_source_command` is off the chain.** It returns before the hook, which is now called from `walk_selected_source_command` (one call site, so it is not also reached from the preparation).
- **The one new frame, `walk_logical_survey_command` (`#[inline(never)]`), holds only:**
  - four `Box`es: target, dispatch, body context, invocation;
  - one `SourceScriptOperands` of about seven words;
  - the by-value `SourceExecutionContext` argument;
  - the `SourceOutcomes` return slot.
- **Non-recursive leaves.** Everything heavy runs in `#[inline(never)]` leaves:
  - `logical_survey_target` (projection);
  - `logical_survey_dispatch`, which holds `InvocationFacts` by value, the `ResolvedInvocation`, the `ClausePlan` and the `ScriptBodyFlow`, and returns a `Box<LogicalSurveyDispatch>`;
  - `logical_survey_quiet_outcome`.

  At most one of these is live, and only at the deepest level.
- **Conclusion.** This is plausibly at or below the Native per-level cost, and so within the 24 KiB envelope. It is not measured. The new depth_guard control and the 2 MiB `pathological_nesting_terminates` are the measurement.

## 3. Reusing the parent context is sound

- **When it applies.** `retained_body_source` reuses the parent's `Arc<OriginalSourceRegistryContext>` only when `realm.source_bindings_ref().source_origin()` (that is, `root_origin`) equals `script.origin`.
- **The bindings are identical in that case.** `SourceCommandBindings::selected_source` (`origin_inventory.rs:529`) starts from `self.clone()` and changes points, accesses and root only when `root_origin != script.origin`. With equal origins it returns the identical bindings.
- **The realm is equivalent.** `original_logical_realm_from_source_bindings` wraps those same bindings with `realm_from_source_bindings(..).with_resolved_analysis_input(input)`, exactly as the parent was built. The only difference v1 produced was a fresh source-advice cache.
- **The same checks still run.** The reuse branch keeps v1's image and input checks:
  - `image ==`;
  - `matches_original_source_image(image, config)`;
  - `matches_resolved_analysis_input`.
- **Decoded bodies are excluded.** A decoded (Materialised / NativeValue) body has a derived origin that is not the root, so it always takes the `selected_source` + `original_logical_realm_from_source_bindings` path, as in v1. A braced body inside a decoded child reuses that child's derived context, which is consistent.
- **Minor gap (non-blocking R-v2-1).** The reuse branch skips the extra guards in `original_logical_realm_from_source_bindings`:
  - a logical advice input is present;
  - the metadata-owner input is equal;
  - the configs are equal;
  - `InvocationMetadataContext::for_analysis_input`.

  None of them can fail for a survey root, which `analyse_package_index_survey` admits only with a retained Logical input. The other root that can be built is the fallback general Logical realm, which records no entered `if` bodies, so `retained_body` already returns `None` before this point. Keeping a cheap `original_logical_source_name_advice_input().is_some()` check here would make fail-closed parity with v1 local.

## 4. Complete-clause-plan rule declines malformed `if`: yes

- **The rule.** `logical_survey_dispatch` requires `schema.clause_plan()?.defect.is_none()` before taking a `Conditional` flow.
- **Where it declines:**
  - `clause_plan()` is `clause_walk()?.ok()`, so it declines for no exact argv (expansion) and for a computed keyword-position word (`Err(ClauseAbstention)`).
  - The `if_` grammar is `expr ?then? body (elseif expr ?then? body)* (?else? body)?`. Its own registry tests cover `MissingBody` (including `elseif 2 then`) and `ExtraWords` (implicit-else plus a trailing word, `then` as an implicit-else body).
- **I re-ran the oracle on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0:**
  - `luna-27/probes/malformed-if.out` is byte-identical in content to Sol-23's `probes/malformed-if.out`.
  - I added an extended probe, `luna-27/probes/malformed-if-extra.{tcl,out}`, which runs each case both as a compiled proc body and through `source` of a file, as `tclPkgUnknown` does.

| Case | Result on all five releases, both modes | Grammar defect |
|---|---|---|
| `else` missing | error `no script following "else"`; nothing ran; `mypkg` unregistered | MissingBody |
| `elseif` missing | error `no expression after "elseif"`; nothing ran; unregistered | MissingBody |
| `elseif 0` with no body | error `no script following "0"`; nothing ran; unregistered | MissingBody |
| `then` missing | error `no script following "then"`; nothing ran; unregistered | MissingBody |
| `else {} extra` | error `extra words after "else" clause`; nothing ran; unregistered | ExtraWords |
| implicit-else plus `extra` | error `extra words after "else" clause`; nothing ran; unregistered | ExtraWords |
| implicit else `if 0 {} {set ::ran 1}` | runs; registers | none |
| `else {}` (complete) | runs; registers | none |

`malformed_first_match_chains_enter_no_body` encodes the first three rows plus the complete control. Adding the `then`-missing and `else … extra` rows would be cheap; it is optional.

## 5. The capture-order test now covers both orders: yes

`conditional_source_control_stays_inside_the_survey` builds one `ResolvedAnalysisInput` per iteration, the way `capture` does, and clones it so that both contexts share the same `Arc` context, and therefore the same memo identity. It then runs:
- `survey_first = true`: `capture_package_index_survey`, then the general `capture`;
- `survey_first = false`: the general `capture` (which retains a memo entry), then the survey.

Each order asserts that the survey realm enters the body and the general realm does not. A new context per iteration keeps the two iterations independent. Constructing `OriginalPackageIndexSourceSurvey { source, ancestors }` directly is allowed, because `package_target_survey` is a child module of `source_structure`.

## R2 and R3, and the Sol-20 documentation condition

- **R2: done.**
  - `analyse_package_index_survey` uses a plain `?`.
  - The two readonly realm builders share a private `readonly_realm` helper.
- **R3: done.** The hook takes `compiled` and declines on `compile_error || unknown`.
- **Facts resolution is stricter than v1.** The dispatch leaf resolves facts through `resolve_structured_invocation_in_realm(..).resolved()?.facts()` and drops `resolve_source_invocation_facts`'s `native_registration_invocation_facts` fallback. That only adds declines.
- **Hook order.** The receiver path now runs before the survey hook. Stock builtins return `None` there, and a receiver claim pre-empting the hook would be conservative.
- **Sol-20 review condition C1 is closed by this patch.** `package-target-source-survey.md` lines 60 and 67 are rewritten, and the patch is built after Sol-20, so line 67's context matches.
  - **Nit.** Line 67's "in production or tests" is broader than needed. The original Walker and the `original.rs` tests still use the lexer word walk `walk_command_words_with_config`, though not as a requirement or script probe. "No lexical requirement or script-geometry probe helper remains" would be exact.

## Still open (unchanged from v1)

- **C2: suites to run.**
  - `pathological_nesting_terminates`, timed against 2.48 s;
  - the new and existing `depth_guard` stack controls;
  - `parse_pkg_index_survives_deeply_nested_source_wrapper`;
  - `package_resolver`;
  - `body_effects` `ordered_condition_uncertainty_tests` and `possible_occupant_lookup_tests`;
  - the O127 / GVN / effect-SSA / side-effect suites;
  - the registry `package_` and `source_navigation` tests;
  - `make rust-check`.
- **C4: expected failures stay explicit.** These are expected to remain FAIL:
  - `retained_control_keeps_actual_body_and_parent_model_lineage`;
  - T174;
  - T046;
  - `the_real_tcllib_try_index_resolves_per_release` (`file::home`).

  The decoded-child advice path is still not implemented. Sol-23's v2 run plan, which traces with `TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS`, is the next step.
