# LSP optimiser release verification

The lane verifies compiler and source-optimiser issues against the current
`work` branch, with fixed-input regressions and the pinned Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0 interpreters. Latest JimTcl is available for applicable
shared syntax checks. The original checkout differs substantially from `work`;
issue status follows this branch's tests, not an older implementation audit.

| Issue | Status | Evidence |
|---|---|---|
| #2216 | Fixed | `line_delete_span_takes_trailing_horizontal_whitespace` checks spaces/tabs and LF/CRLF without consuming the successor. |

The remaining assigned issue witnesses are being checked in
`rust/tcl-compiler/tests/release_optimiser_safety.rs`. A passing witness records
behavioural preservation, including completion and the caught error message;
it does not on its own claim a missed optimisation now fires.

## Integration checkpoint

The initial 44 deterministic witnesses passed 41 cases on all applicable pinned
Tcl interpreters. The residuals were #2334 (a standalone expression became a
command named `3`), #2351 (a scalar write was removed before an array-element
error), and #2434 (a folded interpolation extended a preceding escape). Fixes
for these three issues are committed for the required shared-branch checkpoint;
the corrected differential run is pending. #2216 has a fixed-input helper
regression; #2439 filters hint, duplicate, invalid and unchanged rewrite rows
from the applied report and has a focused multipass regression. Formatting and
`git diff --check` completed; Rust compilation and push gates remain pending at
this checkpoint. No failing test has been removed or weakened.

## Verified first batch

`cargo test --offline -p tcl-compiler --test release_optimiser_safety -- --nocapture`
passed all 44 initial issue witnesses, with no ignored tests, after the first
fix batch (`1c15d4ff5`). The suite used every applicable pinned Tcl 8.4–9.1
oracle. Each test is named `issue_<number>` and compares stdout, stderr,
process completion, Tcl completion and the caught result/error message.

| Issue | Current implementation | Validation |
|---|---|---|
| #2334 | Fold the original expression operand and retain the command invocation. | `issue_2334` passed all five Tcl releases. |
| #2351 | Treat a later same-root array definition as observing scalar shape. | `issue_2351` passed all five Tcl releases. |
| #2434 | Keep interpolation substitution boundaries when a shared escape scan would cross them. | Initial `issue_2434` passed all five Tcl releases; broader extent/Jim controls queued. |
| #2439, #2358, #2406 | Report changed source edits, excluding hint-only, invalid, duplicate and unchanged candidates. | Focused multipass and exact public-pipeline report controls queued. |
| #2341 | An always-false `if` names its consequent as unreachable. | Diagnostic assertion queued; existing wording assertion corrected. |
| #2384 | Select the retained loop's actual entry header, including a synthetic rotated header. | Rotated-loop and zero-iteration control assertions queued. |
| #2402 | Read all registry-declared loop scripts, declining certainty for dynamic words and accounting for exits from init/step scripts. | Dynamic body, exiting init and literal infinite-loop control assertions queued. |
| #2093 | Keep the documented conservative policy for relative `[self class]`; its receiver-local target is unproved. | Natural-spelling preservation assertion queued. |
| #1797, #2218, #2373, #2411, #2417, #2419, #2431 | Verify the current generation, trust, numeric-consensus, recursion, root-identity and Tcllib contracts. | Expanded controls queued after the priority #940 build. |

The expanded suite also covers the additional examples in #2231, #2258,
#2260–#2262, #2308, #2340, #2390, #2400 and #2430. Tcllib 2.0 is supplied by
`TCLLIB_2_0_DIR`; all six reported mutators were independently checked against
Tcl 8.5–9.1 before their optimiser differential run. Compilation of the first
expanded test draft found a test-only enum iteration typo, since corrected.
The next shared-branch checkpoint precedes the queued expanded verification
and push gates, as explicitly required by the integration cadence.
