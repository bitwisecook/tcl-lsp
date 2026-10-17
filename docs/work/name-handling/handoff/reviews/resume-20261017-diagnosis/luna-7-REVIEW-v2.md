# Luna-7 re-review: Sol-5 `same-document-order-v2.patch`

**Verdict: HOLD.** One item blocks admission, and it is narrow.

v2 fixes H1, H2, M1 and M2. Its handling of M3 is sound for every positioned query. But reading the document end as "after load" can still revoke a removal on an exit path that v2 does not model. That fails the orchestrator's ruling. Probes below show it for errors and for ordinary early completions.

The fix uses registry data only and keeps the target flips for tests 400, 404 and 406 (see "Required fix").

## Review basis

- **Patch:** sha256 `6d9d2ed0200518a57d926349f16eb1fbf33d3afd51180d2b81b0d362a5d6461e`.
- **Baseline:** the `a/` copies are byte-identical to the current HEAD `89cfb925d`, and `git apply --check` passes.
- **Method:** static review only. Nothing was compiled or run.
- **New probes:** tclsh 8.6 and 9.0, in `scratchpad/agents/luna-7/probe/v2/run.tcl`. Both versions agree.
- **Line references:** `b/` means `scratchpad/agents/sol-5/b/`.

## M3 ruling check: reach-conditioned ordering

### Positioned queries: sound (PASS)

A query at a command position is reached only if control passed through every earlier straight-line position of its document:

- The straight regions are the root and the bodies entered by real built-in `namespace eval` or `eval`. No `catch` or `try` body is ever straight.
- So any error, normal early exit or `source`-time error before the query makes the query unreachable, and the relation is vacuous there.

Errors I checked against this argument:

- **Top-level errors with no `catch`:** they propagate out of the document. `probe/qual.tcl` and `probe/conflict.tcl` show the later statements never run.
- **Errors raised by a `source`d file:** they propagate into the sourcing document the same way.

No production consumer builds a Modeled-purpose query at or past a document end:

- I grepped tcl-lsp-core and tcl-lsp-server for `u32::MAX` and `.len()` used in `RunPoint`/`CallSite` construction. Every hit is a test.
- Production queries are call sites and import sites, which are command positions.

Positioned soundness also needs the entering command to be the real built-in. The residual risk there is low:

- The M1 guards cover shadowing in this document.
- They do not cover a `proc eval` in another document sourced earlier, `rename myeval eval`, or `interp alias {} eval`, unless the realm barrier refuses `Selected` words. I would add one control for `rename`.

### The document end: not sound under either reading (FAIL)

In `contains` (`b/rust/tcl-lsp-core/src/source_graph.rs:1093-1133`), any `at >= end` is a sequence position unless `cutoff` is set. §8.1 calls it reach-conditioned, but the code mixes two readings.

**Reading 1: the end means control falls off the end of the text.**

- Every early exit is then vacuous, and the `return`/`exit` cutoff is unnecessary.
- But the answer at the end would then not mean "after load". 404's `u32::MAX` and test-400's `source.len()` are used to mean "after load".

**Reading 2: the end means the document completed normally.** This is the reading the `return` cutoff implements, and the one consumers rely on.

Under this reading every normal-completion exit must be modelled, and v2 misses these. tclsh 8.6 and 9.0, `probe/v2/run.tcl`:

| shape | source rc | `::dst::p` after load | v2 at the document end |
|---|---|---|---|
| `source y.tcl` where y.tcl does `return -code return` (x_src) | 0 (also LOADED via a proc loader) | alive; the forget never ran | revoked |
| `source y.tcl` where y.tcl does `return -level 2` (x_lvl) | 0 | alive | revoked |
| user `proc myreturn {} {return -code return}`, called at top level (x_proc) | 0 | alive | revoked |
| top-level `break` (x_brk), sourced inside a loop | loop rc 0 | alive | revoked |

The `break` row also applies to `continue`. Both carry `Traits::TRANSFERS_CONTROL` (`rust/tcl-registry/src/commands/tcl/break_.rs:43-47`, `continue_.rs:43-47`) but not `TERMINATES_BLOCK`. v2 checks only `TERMINATES_BLOCK | TERMINATES_PROCESS` (`b/source_graph.rs:802-830`).

The unmodelled paths in detail:

- `source` has a `SourceFile` body execution, so its flow is `None` and `runs_now` is false. It never cuts off.
- A command whose words are unresolved or not validated never cuts off (`b/source_graph.rs:~990`, `if let (Some(words), Some(flow))`). That covers user procedures and dynamic heads.
- A `return` inside a root `namespace eval` body is a terminator only if `source_registry_words` resolves inside that caller-context-unavailable body. NOTES risk 1 says this is unverified. If it does not resolve, `namespace eval dst {return}` goes unmodelled: tclsh `x_nsret` shows the alias alive, while v2 would revoke at the end.

**Reading 3: the end means after the load attempt.** This is the orchestrator's error-path condition, for example a `catch {source x}` loader.

- `probe/qual.tcl`: `namespace export p; namespace export ::q::r; namespace export -clear` raises an error and leaves `p` exported. v2 answers NotExported at the end.
- The same happens with `probe/conflict.tcl`.
- §8.1 concedes this ("only an 'after load' reading of the document end can differ"). As literally stated, the orchestrator's ruling is therefore not met.

### Required fix (registry data only; positioned queries unchanged)

1. **Define the document end in §8.1 as "after the document completes normally".** An error path never reaches it, which satisfies the ruling for errors. Also state that no consumer may read the end as "after a failed load". Cross-document consumers are Conditional anyway, and no production consumer queries the end today.
2. **Add a separate `end_cutoff`.** It withholds only the document end, not positions. Set it at the first command outside definition bodies, in a straight region or nested in a straight command's non-deferred regions, that:
   - has `TERMINATES_*` or `TRANSFERS_CONTROL` traits; or
   - has words that are unresolved, unvalidated or not `Selected`, which covers user procedures, dynamic heads and unresolved commands in `namespace eval` bodies; or
   - selects `BodyExecutionSpec::SourceFile`.

   Tests 400, 404 and 406 use only built-ins (`proc`, `namespace eval/import/forget/export`), so they still flip, provided their words come back `Selected`, which risk 1 already requires.
3. **Add controls:** x_src, x_proc and x_brk must keep the alias, or the export, at the end. Keep a positioned query before the exit `KNOWN`, to show positioned ordering is unaffected.

If the orchestrator wants reading 3 instead, the document end can never be admitted, because the registry has no "cannot raise" fact. Tests 400 and 404 would then stay red. That is a user-level trade-off and should be escalated rather than patched.

## Resolved findings

- **H1 (PASS).** `alias_certainly_live` (`b/rust/tcl-lsp-core/src/namespace_import.rs:787-814`) is wired into `conflicting_alias_at` through `alias_certainly_live_at` (`b/workspace_index.rs:8645-8730`).
  - The conflict needs an install known to have run at the later import.
  - Every removal must be known not to have run at the later import, or known to have run before the install. An unplaced removal (cross-document, body-local, branch, destroy) leaves the alias possibly dead.
  - This is sound under reach-conditioning: if I1 itself conflicted at top level, I2 is unreachable.
  - Controls: `a_sourced_forget_between_two_imports_does_not_make_the_later_one_conflict` and `a_branch_forget_between_two_imports_does_not_make_the_later_one_conflict`, at `b/workspace_index.rs:12781-12830`. Both match my probes.
  - Constructed models and SourceHorizon fall back unchanged.
- **H2 (PASS).** `removal_in_effect` uses `query.is_some_and(..)` (`b/namespace_import.rs:202-215`). With no query point, no removal applies on source navigation, which equals HEAD for exact and glob imports (`link_alias_live`).
  - Exact links still diverge from glob imports for a positioned caller, but in the conservative direction: the exact link is kept.
  - Low: the `link_alias_live` doc (`b/workspace_index.rs:8737-8746`) still says "every recorded removal counts as having run" and that a later forget "revokes the link", which contradicts the new bullet. Reword it as constructed-model behaviour.
- **M1 (PASS).** Body entry now requires all of the following (`b/source_graph.rs:861-887, 990-1030`):
  - the command is outside `caller_context_unavailable_bodies`;
  - its words are `Selected`;
  - no procedure or class in the document declares its head name.

  Non-head points also need `Selected` words with complete roles. Controls cover a namespace-local `proc eval`, a namespace-local `proc namespace` and a global `proc eval`. The residual cross-document hijack risk is noted above.
- **M2 (PASS).** `enters_once` requires `frame_effect` to be absent or to have `FrameLevelWord::None`. `eval` has `None` (`eval_.rs:87-88`), `namespace eval` has no `frame_effect`, and `uplevel` has a level word (`uplevel_.rs:171`). An `uplevel #0` control is included.
- **M4 (PASS, with low items).**
  - §8 is corrected, and §8.1 is rewritten without dated ids.
  - The proof doc is folded into `scope`, and two linked bindings are added.
  - Contract row 631 and three owner rows are updated, and workspace-indexing.md gets a pointer.
  - Still to fix: the §8.1 document-end claim (above), the `link_alias_live` doc (H2), and `b/workspace_index.rs:8797`, which still cites `definition::exported_original_slot_at_import`. That function is now `#[cfg(test)]`; production uses `exported_original_slot_at_import_in`.
  - Unverified gates: the proof gate, owner-resolution and clippy.
- **M5 (improved; timing still needed).**
  - Capture is a single start-ordered sweep with parent links and per-region command lists.
  - `contains` uses `partition_point` and a parent walk.
  - The definition tier shares one `OnceCell<RunOrder>` per `live_import_at`, so there are fewer captures than at HEAD.
  - The extra work over HEAD is words for commands in non-definition caller-context-unavailable bodies, plus one `with_source_schema` per command.
  - It still needs timing: `settling_invocations_does_not_scale_with_imports_times_exports` (< 2 s), test-172, and references on a large corpus.
- **L1 (PASS).** `a_clear_in_the_sourcing_document_stays_conditional_for_the_sourced_export` is added. It pins the decision's Conditional answer, `Exported`, although Tcl itself gives NotExported. That is acceptable by decision; record it as decision-encoded.
- **L2 (PASS, with a low item).**
  - The test is split, and the slot is now an enum.
  - Low: `DocumentSequence::capture`'s body is about 112 code lines (`b/source_graph.rs:937-1051`), not the ~90 NOTES says, so `clippy::too_many_lines` may fire. HEAD already has longer non-test functions without an allow (`workspace_index.rs` `index_document`, 234 lines; `definition_with`, 163), so the gate status is unclear. Splitting out the terminator and entry step is cleaner anyway.
- **L3 (PASS).** Definition bodies are refused through `in_bodies`, with a running maximum.

## Compile and rules

Every new API is present at HEAD, with compatible visibility and `Copy`:

| crate | API |
|---|---|
| `tcl-registry` | `ResolvedInvocation.semantics.{traits, frame_effect}` (pub); `FrameEffectSpec` (`Copy`); `FrameLevelWord` (`PartialEq`, `pub mod frame_effect`); `Traits::intersects` |
| `tcl-compiler` | `written_argument_roles`; `SourceDeclarationMetadata::{name, metadata}`; `ProcDef`/`ClassDef.body_span` |
| `tcl-core-types` | `NameBytes::as_bytes` |
| `tcl-syntax` | `key_holder_and_tail` |
| `tcl-lsp-core` | `rustc-hash` dependency; `#[cfg(test)]` wrappers have only test callers |

- `while let` / `if let` chains need edition 2024, which the crate uses.
- No `#[allow]` and no command-name matching was added.

## Control scope

- Only the same-document shape changes, and every other assertion is unchanged.
- The second and third 1b45c540e controls still need sign-off.
