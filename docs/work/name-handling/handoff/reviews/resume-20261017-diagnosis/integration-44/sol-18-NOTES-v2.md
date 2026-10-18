# Sol-18 v2: response to Luna-20

Base: HEAD `9cc6b4089`. I ran no cargo, made no writes to the repo and used no worktrees.

Every new patch passes two checks:
- `git apply --check` on a clean copy of its `a/` tree.
- `git apply --check --cached` in `/home/user/tcl-lsp`.

rustfmt (edition 2024) reports zero diff hunks for every changed `.rs` file, before and after the patch. No patch adds an `#[allow]`.

Scratch root: `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-18/`

| # | Patch | Status | Trees |
|---|---|---|---|
| 1 | `compiler221-v2.patch` | Ready. v1 plus the proof-doc update. | `p1v2/` |
| 2 | — | **No sound v3 drafted.** Blocker below. `db921-v2.patch` stays HELD. | — |
| 3 | `vsatisfies-84-v2.patch` | Ready. Arity is now three-state and aligned with its owner. | `p3v2/` |
| 4 | `forced-import-assertion-v2.patch` | Ready. Static trace below. The query now sits on the call head. | `p4v2/` |

The tclsh evidence is unchanged from `NOTES.md` and `probes/`.

## 1. compiler221-v2

`compiler221-v2.patch` changes two files.

- **Test.** `source_class.rs:1081` changes as in v1: `is_none` becomes `is_some`, with a tclsh comment.
- **Proof doc.** `docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md` changes in two places.
  - Line 18 now says the original class cards "do not themselves supply a completed command world; an independently completed original command world is retained beside them". It cites tclsh 8.6.18, 9.0.4 and 9.1.0: code 0, one fixed world.
  - Line 140 now says "Lexical edit admission remains unavailable; the independently completed original command world is retained", with the same oracle.

Neither sentence appears in `manifest.json` or any other tracked file; I checked with `git grep`.

**Unchecked:** whether a proof or hash drift gate covers this document's bytes. If one does, it needs a regeneration that I cannot run here.

## 2. db921 v3: blocker, no patch

I could not make v3 sound statically. The three constraints Luna-20 set cannot all be met by changing the merge predicate.

**The test failure is a unit count, not a duplicate worker request.**
- `sol-nextto-db/runs/db921.log` fails at `lib.rs:12034`: `assert_eq!(unit.procedures.len() + unit.body_units.len(), 1)`, with left 2 and right 1.
- The assertion just before it, `checked == 1`, passes. The lattice worker is already asked only once, so the analysis already treats the proc as one request.

**The two units are two different lowerings. A single unit cannot carry both.**

The authored unit:
- is lowered with `readonly_authored_body = true` and `SourceBodySyntaxOwner::Authored`;
- has `executed_source == None`, which `OriginalAuthoredProcedureBody::matches_procedure` requires;
- uses the rebased original CFG path (`original_procedure_cfg_for_body`).

The Installed unit:
- is lowered by `retain_installed_body` with `executed_source = Some`;
- uses the full-config CFG (`installed_procedure_cfg_for_body`);
- carries the Source SSA summaries (`matches_installed_procedure_summary`).

`ReadonlyAuthoredCoverage::capture` explicitly refuses an id that appears in both maps.

So "one surviving identity that keeps the allocation and executed source reachable" has only two realisations, and neither is sound statically:

- **(a) Keep the authored unit and suppress the Installed one** (what v1/v2 did). Every consumer of the Installed unit loses the body:
  - `object_types::procedure_implementation_units` pairs by `executed_source`;
  - `installed_procedure_cfg_for_body`;
  - `RetainedSourceModuleBindings` installed summaries, which is the Source SSA identity;
  - `analytical_closed_projection`'s `source_body_inventory`.

  Making those consumers accept the covers relation would feed them the readonly authored lowering in place of the executed one. Whether those results are equal cannot be established without running them, so this route has not been shown to preserve Source SSA identity.
- **(b) Keep the Installed unit and suppress the authored one.** Every consumer that reads an authored card by id loses it, because `matches_procedure` rejects an Installed body. Those consumers include `interprocedural`, `ssa`, `authored_controls`, `declaration_preview`, `body_analysis_capsule` and `analysis_body_correspondence`.

**What C Tcl decides.** C Tcl requires one command per evaluated `proc` (`probes/db921.out`). It does not decide how many analysis units represent that command.

The code already models the one-identity rule as a relation. `covers_installed_analysis_source`, through `ReadonlyAuthoredCoverage`, uniquely pairs each Installed unit with one authored unit. The three `readonly_authored_coverage` tests pin that relation, and they agree with tclsh. The only assertion that conflicts is the DB test's internal representation count, which tclsh does not settle.

**Decision needed from the user.** The 11:55Z application assumed duplicate suppression is sound. Luna-20 shows it erases the Installed and Source SSA identity. There are two ways forward:

1. **Keep both units and correct the DB count.** The coverage relation already expresses C Tcl's one identity, so the DB test would count identities, not units. This needs a small public query from tcl-compiler, such as "Installed units uniquely covered by an authored unit". `ReadonlyAuthoredCoverage` is `pub(super)`, so the DB crate cannot see it today. The coverage tests stay unchanged, and no consumer changes.
2. **Fund a merged-unit design.** One unit would carry both lowerings, or the readonly lowering would be shown equivalent for every Installed consumer. This needs measured runs of the coverage, interprocedural, `object_types`, `source_graph`, SSA and DB tests. It cannot be done statically.

I have not drafted either option. Option 1 contradicts the literal text of the 11:55Z application, so it needs a ruling first.

## 3. vsatisfies-84-v2

`vsatisfies-84-v2.patch` changes three things.

- **Registry `ResolvedInvocation::arity_accepts_under`** is now built on `authored_source_arity()`.
  - It declines with `None` wherever that owner declines: an unresolved subcommand selector, or `STRUCTURALLY_CHECKED_ARITY`.
  - It also returns `None` for non-empty package `arity_windows`.
  - Each admitted form recounts through `authored_source_count_for_arity(form.arity)`. An indeterminate count is `None`.
  - With no form admitted, the common arity decides.
  - Across admitted forms: any `Some(true)` wins; otherwise any `None` gives `None`; otherwise `Some(false)`.
- **Compiler receipt.** It now stores `release_arity: Vec<(TclVersion, bool)>` for decided releases only, and `argv_accepted_in(target) -> Option<bool>`.
- **`Condition::holds`.** `Some(true)` continues as before, `Some(false)` gives `No` (C raises, as in v1), and `None` gives `Inert`. An unknown count, or a missing `authoring_query`, never becomes a definite Unavailable.

The tests from v1 are kept. The receipt test now asserts `Some(false)` and `Some(true)` exactly.

**Not run:** `reachability::tests`, the `package_` registry tests and the `package_target_survey` tests.

## 4. forced-import-assertion-v2: static trace

The test's analysis is `Analyser::new().analyse(src, "tcl8.6")`.
- `tcl8.6` has a native name protocol, so `has_logical_source_name_context()` is false and `allows_lexical_declaration_advice()` is false. The compiler221 sibling asserts the same for this constructor and dialect.
- So `resolve_called_proc` returns through `original_declaration::select_at_offset` (`definition.rs:3926-3932`). It never reaches `forced_import_shadows`, `live_import_at` or `proc_visible_from_namespace`.

**Luna-20's suspected route is unreachable from `resolve_called_proc` at HEAD.**
- `allows_retained_logical_declaration_advice()` simply returns `allows_lexical_declaration_advice()` (`analyser/types.rs:3134`).
- Every analysis therefore returns at one of the first two `if`s: either through original selection or through `logical_called_procedure_at`.
- The forced-shadow, namespace and wildcard tail in `definition.rs:3938-4006` is dead code for this function.
- The defect Luna-20 describes is real in that code: after a forget, `live_import_at` sees the alias removed, so it would return the stale local. But fixing it would add only dead code, so I did not draft one. That tail is worth a separate cleanup task.

**The live Native route, statically, gives `None`, which matches tclsh.**
- `apply_import` with `force = Some(true)` and a fully matching export set does not extend `prior`. `::dst::p` becomes `{Imported(::src::p)}`, so the local binding is replaced (`original_namespace_binding.rs:156-257`).
- `apply_forget` turns that `Imported` binding into `Missing` (`:259-322`).
- In `select`, `matching_source_advice_publications` first picks the `::dst::p` header. The positioned state's explicit binding for that slot is `{Missing}`, so `agrees` is false, nothing is admitted, and the result is not exactly one, so `None`. A positioned resolved reference cannot target `::dst::p` for the same reason.
- `OriginalCommandWorld::import` (forced `publish` replaces the slot) and `forget` (removes the Imported publication) agree: the completed world holds no `::dst::p`.
- No implementation fix is needed for this route.

**The problem I found instead: the test was vacuous.**
- `after_last(src, "    p\n")` points one past the call's line, at the closing `}` of `namespace eval dst {…}`.
- The Native selection picks the `namespace eval` invocation there and can never answer a proc. The assertion therefore held whatever the implementation did.
- v2 keeps the fixture and the `is_none()` assertion and moves the query onto the call's head word: `after_last(…) - 2`. I checked that this offset lands on `p`. That exercises the real `p` lookup.

**The same problem affects other tests.** The positive control `a_forced_import_shadows_the_local_command` (`definition.rs:6350`) and the other import-lifecycle tests use the same past-the-call offset. On the Native route they cannot select the call. That is consistent with their FAILED status in `sol-5/dbg/base_def.txt`. Contrary to the review, the positive control does not currently stop the suite being vacuous. This is outside these patches but should be fixed alongside them.
