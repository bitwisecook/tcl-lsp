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

## Fresh external validation: #2507

`issue_2507` in `tcl-lsp-core` passed all seven integration tests at
`ef032bdb7`, including the implementation fix `14b26b2e0`. The harness retains
the actual registry/input, checks diagnostic edit currency, and runs the real
W100, O111, report and code-action producers. The posted RGB example retains
three W100 findings, three O111 hints and three scan recommendations under
Tcl 8.4–9.1 and Jim profiles.

The initial current-source action check exposed six invalid edits: three
quick fixes and three bracing refactors. Direct bracing raises on all six
interpreters. The fix withholds that edit for the recognised hexadecimal
conversion while preserving the warnings and recommendation. An ordinary
arithmetic brace action still applies and executes correctly.

Four RGB inputs have explicit expected outputs and matching expr/scan results
under Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and current Jim. Three further
inputs (`-ff`, `ff+1`, `zz`) demonstrate why unknown hexadecimal text cannot
justify an automatic scan replacement. Twelve source-pattern controls include
quoted/uppercase prefixes, ordinary arithmetic, multiple substitutions,
existing braces, and a dynamic command head; a user-defined `expr` control
receives no conversion recommendation.

The Jim checkout is `82fb4d0caf90240d94352a31fcbd16adf52658a7`; its built binary
and the installed oracle have identical SHA-256
`0fd5a727a6b4c0856818e9237e3e962fe1062c4057bcb5b3f768efd768cfae52`.
The binary reports version/patchlevel 0.84. Tests discover it through the
shared oracle owner. Machine-readable receipts are in
`/tmp/lsp-2507-evidence.json` and `/tmp/lsp-2507-oracles.json`.

Targeted Clippy exposed two earlier #2434 lint defects, corrected mechanically
in `2066b20c3` without changing escape behaviour or adding an allow. The next
Clippy attempt reached an unrelated redundant closure at
`original_command_rename/source_coverage.rs:596`; its owner has been notified.
Both selected #2434 regressions passed after that mechanical change (Tcl
8.4–9.1 plus the applicable Jim escape controls). Older #2373, report-allocation
and Tcllib setup work remains safely stashed as
`Preserve optimiser release WIP before fresh issue 2507 verification`.


## Resumed compiler safety: #2218

The existing `issue_2218` regression fails at `182ea7784`: O115 still rewrites
`double_expr` beside `proc g {cmd} {$cmd}`. Direct owner probes show both the
unit's and a freshly scanned command-mutation fact already report dynamic
mutation and distrust `expr`; the propagation helper bypasses that fact.
The implementation now retains the registry-resolved expression invocation
and checks its descriptor against the whole-module trust gate before O115.
No binding-owner, CLI, VM or F5 code is changed. Focused return/argument
controls and execution under Tcl 8.4–9.1/current Jim passed all three focused
regressions at clean `c47d83bda` (15.60 seconds). Eight positive and eight
negative return/argument controls include a proved computed `puts` head.
The six-engine execution installs a mutating target after load, dispatches it
through the public dynamic procedure and compares the resulting output.
The sample README's dynamic-head row now says **not reported** and its former
remaining-gap paragraph has been replaced. A rerun with the refreshed strict
Jim reference and targeted Clippy is in progress. Older tooling
and allocation/setup drafts remain stashed.
