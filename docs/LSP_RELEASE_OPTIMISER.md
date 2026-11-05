# LSP optimiser release verification

The current #2426 diagnostic-payload audit and the two verified typed parse-cut
consumer fixes are recorded in
[LSP_RELEASE_DIAGNOSTIC_PAYLOAD_AUDIT.md](LSP_RELEASE_DIAGNOSTIC_PAYLOAD_AUDIT.md).
The audit is scoped to LSP/compiler consumers; it does not claim the full issue
is fixed. All seven exact compiler controls and production compiler Clippy
passed; the remaining consumers and coverage gaps keep #2426 open.

The next optimiser arbitration residual is tracked in
[LSP_RELEASE_OPTIMISER_SELECTION_AUDIT.md](LSP_RELEASE_OPTIMISER_SELECTION_AUDIT.md).
Its conservative tie-refusal candidate is local and unverified; fixed-workload
impact counts are required before it can land.

Priority #940 now takes this lane: a separate registry child is being prepared
with the values lane for nonempty list-append container representations. The
approved interface retains the selected handler/profile/shape and states only
a conditional List requirement/store/result for the target container. Ordinary
objects, unobserved storage, original source/receiver custody and successful
handling remain separate premises; appended element representations are never
copied to the target. Latest corrections remain under `/tmp` for the values lane's coordinated
integration. Independent review added agreement between the
selected native operation and the effective semantic operation; refusal
fixtures include an actual selected form hiding raw extra effects. The frozen
child is `/tmp/lsp-940-conditional_list_append.rs`, SHA-256
`d402002e415b27516753e8e9a9b16c1a9d876ceb1c9aec11005eddc8c9739f85`.
Its latest ten fixed-input Rust controls and compiler integration remain
unverified. The previous frozen six-test run passed five and exposed the
catch-all shadow fixture selecting the scoped builtin; the corrected fixture
retains matching availability and asserts actual generic selection first. The
prior List requirement does not certify physical coercion of the existing
object or justify a new append S100 finding.
The final narrow append boundary patch also checks raw world/state absence
and excludes NUL from definite malformed-list proof. Its added control uses an
actual altered registry descriptor followed by an explicit public effective
carrier reset; no ordinary constructor defect is claimed. The patch and exact
base/new hashes are retained in `/tmp/lsp-940-append-raw-boundary.json`.
The latest isolated direct-kernel proposal is
`/tmp/lsp-940-append-direct-phase.patch`, indexed by its adjacent JSON receipt.
It names the earlier captured receiver's current target read after ordered
arguments, the evaluated value range and conditional lookup preservation.
The exact original stock trait bits are shared with the declaration; raw and
effective traits, frame/body metadata, native compilation and successful
handler contracts must agree. Generic facts/effects and Native/Normal authority
stay unchanged, and no whole-handler quietness or append S100 is granted.
This proposal is under static review and has not passed a fresh runtime gate.

The actual registry failures are preserved in
`/tmp/lsp-940-list2-conditional_list_append.log` (5/6) and
`/tmp/lsp-940-list2-conditional_list_index.log` (6/7). The latter's old
all-domain transition intentionally triggers command-trace callbacks, so its
no-callback precondition was false. The test-only correction keeps that
callback-bearing refusal and adds a genuine VariableCells-only no-callback
case. All cases are labelled, no refusal was weakened, and production prefixes
are byte-identical for that correction. Exact patches and hashes are in
`/tmp/lsp-940-list-registry-control-correction.json`; corrected tests are unrun.
The read-only foreach initial-phase audit is retained in
`/tmp/lsp-940-foreach-initial-phase-audit.md`. C9 numeric cache preservation
precludes an unconditional physical List conversion claim; that route is
parked while append is prioritised.

The handoff manifest is `/tmp/lsp-940-conditional-list-append-manifest.json`.
It links pinned C source reviews and four standalone Tcl execution records,
including the initial C9 trace-syntax failures. The portable observer/shadow
script and separate root/procedure operand-order scripts each completed under
all five pinned C releases and current Jim. Representation introspection was
unavailable under C8.4, C8.5 and Jim, so those runs establish only their recorded
values and errors. The shared evidence checker passes all four records, and
the extractor recovers each exact executed script. These native observations
do not verify the LSP projection. The target-cell read must follow all appended
operand effects while retaining the earlier receiver-name value; neither
appended numeric representations nor stale pre-operand producers can supply
the target container representation. No shared values-tree write or Cargo
invocation is claimed for this checkpoint; full push gates remain pending.

The follow-up original-report scout is recorded in
`/tmp/lsp-940-conditional-if-seam.md`. Existing conditional clause/body and
error-exit topology can be reused, but a separate source-control phase must
retain exact nested-body custody and expression conversion effects. The
source-only expression parser cannot supply Native preparation. No if source
change is implemented. A separate temporary registry child,
`/tmp/lsp-940-conditional_list_index.rs` (SHA-256
`c5d9d5f284a3f1f5b5616a9aed31b6bcbf44232bc886867373a68b5c4b57f6d6`),
projects one fixed literal index through the shared list/index owners. Unknown
container bytes remain unknown and numeric-looking elements establish no
result intrep. Independent review found that the world-barrier query excludes
only reentrant callbacks; the revised child also requires raw and effective
explicit empty world/state descriptors. Its seven fixed-input controls include
real registry-clone non-callback write/transition refusals and remain unrun;
only rustfmt passes. The current registry prerequisite correction also requires
the exact raw/effective stock representation effects for both children. The
index predicate delegates definite ASCII/no-NUL malformed-list counterevidence
to the shared parser; unknown input and encoding abstain. Actual altered-spec,
effective-only and hidden-raw controls are prepared. The exact patch and base
hashes are in `/tmp/lsp-940-list-registry-prerequisites.json`; values owns its
shared integration. The child event diff was reviewed statically for current
target ordering, stale provenance and captured literal handling, with no
concrete finding; no runtime validation is claimed. The manifest is
`/tmp/lsp-940-conditional-list-index-manifest.json`. The original report still
requires genuine typed append stores and condition/body coverage alongside
an active repeated-conversion control; quiet diagnostics alone are not proof.
Nine unexecuted condition/substitution/alias/error/return/lazy/store designs are
retained in `/tmp/lsp-940-conditional-if-controls-design.json`. The native
expression parser is only a potential gap for actual unversioned/logical
inputs: no pinned original-report parse refusal was reproduced. Any new
source-expression fallback remains parked while existing checked-condition and
effect topology are reused first.

The next reviewed if seam is a temporary field-only prototype:
`/tmp/lsp-940-source_conditional_control.rs`, SHA-256
`cba08f26397b382d6a562e889cad34b574825c7203522c555a26d30abf6532df`.
It adds an opaque default-None source-control descriptor without changing
`SemanticsDeclaration`, `InvocationSemantics` or `InvocationFacts`. Exact
literal condition/body values and the existing clause ordinals are retained
in an owned receipt; outer dynamic/expanded words decline. Six unrun fixed
controls include unchanged generic None/effects/completion/body topology,
literal0/1 and lazy/substitution text, raw/effective descriptor refusals,
receipt lifetime and direct representation-effect overrides. The latter keeps
the actual selected source marker while adding a registry-declared list
coercion; exact raw None/effective None guards withdraw the receipt. Conditions
and bodies retain their separate representation effects. The seven-file metadata wiring includes an explicit SpecTcl
GAPS exclusion and passes apply-check; child rustfmt passes. All product source
remains under `/tmp`, with no Cargo or new Tcl execution. Paths and hashes are
in `/tmp/lsp-940-if-field-manifest.json`. Exact distinct if-body Job custody
and condition conversion/exit ordering remain the values lane's obligations;
fixed boolean truth does not establish representation-inert condition objects.
The exact parent-body design is retained separately in
`/tmp/lsp-940-if-body-custody-design.md`; it reuses the existing common context
owner and authentic immediate-parent catalogue chain, with a distinct If
protocol and no reconstructed root or blind nested-Job inheritance. It has not
been implemented. The prior test-only receipt is preserved under
`/tmp/lsp-940-source_conditional_control-516d3e07.rs`; the pre-correction source
is also retained as `/tmp/lsp-940-source_conditional_control-4b6dded5.rs`.

The lane verifies compiler and source-optimiser issues against the current
`work` branch, with fixed-input regressions and the pinned Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0 interpreters. Latest JimTcl is available for applicable
shared syntax checks. The original checkout differs substantially from `work`;
issue status follows this branch's tests, not an older implementation audit.

| Issue | Status | Evidence |
|---|---|---|
| #2216 | Fixed | `line_delete_span_takes_trailing_horizontal_whitespace` checks spaces/tabs and LF/CRLF without consuming the successor. |

The remaining assigned issue witnesses are being checked in
`rust/tcl-codegen/tests/it_3/release_optimiser_safety.rs`. A passing witness records
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
remaining-gap paragraph has been replaced. The existing test binary also passed its single six-engine execution case
with the refreshed Jim oracle (0.84-10-g82fb4d0), outside the build wrapper.
The equivalent compilation rerun was cancelled to release the priority #940
slot. The coordinator verified production lexer/compiler Clippy at
`c80aa092d`, which includes the implementation fix; no duplicate library
Clippy or unrelated regression matrix was run. The focused proof draft and
machine-readable receipt are `/tmp/lsp-2218-comment.md` and
`/tmp/lsp-2218-evidence.json`. Older tooling
and allocation/setup drafts remain stashed.


The coordinator verified authorship, posted and fetched back the
[proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2218#issuecomment-6630388414),
and closed #2218. The verified implementation is
`c47d83bdadd7ee63be11be7b2b633a6972178064`. The optional test-only required-Jim
followup `8f1c0861c` was not recompiled; it is separate from the passing frozen
three-test implementation proof and the current-Jim execution run. This lane
performed no external comment or closure action.


## #2384: selected loop heads verified; computed entry remains open

The source-selected loop-head defect is fixed in `d1a433d31` and `04a53559b`.
A user procedure named `while` previously ran once and still received W240,
beside its correct W113 shadow warning. The producer now consumes the actual
selected registry schema's loop trait, complete roles, accepted source arity
and iteration plan. Explicit document loop contracts retain their separate
source-declaration path and do not borrow a same-spelled builtin counter plan.
No command-binding, SSA, VM, CLI or F5 implementation is changed.

At clean `993a908ffd682982832621d734f61ffc7c1e8099`, the baseline was 3 passed,
2 failed: computed entry and the shadowed head. After the fix, a clean rebuild
at `eb15be0828cf4cc8d3961f970f6bfa4faf11e389` passed in 3m38s; the emitted binary
ran outside the compiler wrapper with strict pinned Tcl/Jim requirements and
one test thread: **5 passed, 1 failed, 0 ignored** in 82.34s.

| Focused test | Result and scope |
|---|---|
| `issue_2384` | Passed all 25 reported rotated-loop diagnostic cases. |
| `issue_2384_literal_false_loop_is_still_reported` | Passed all 18 builtin/explicit-stub W240 controls across five Tcl profiles and Jim. |
| `issue_2384_loop_execution` | Passed all 36 exact-output original/rewrite comparisons under Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-10-g82fb4d0. |
| `issue_2384_qualified_and_alias_loop_heads` | Passed all 35 qualified, alias, captured-prefix and replacement-head checks under five Tcl profiles. |
| `issue_2384_shadowed_loop_heads_are_not_reported` | Passed all 24 user-procedure head checks under five Tcl profiles and Jim. |
| `issue_2384_computed_false_loop_is_reported` | Still fails: the exact documented zero-entry counter condition produces no W240. Assertion unchanged and separate. |

#2384 remains open. The same-path computed-entry positive is required before
claiming the rotated-condition diagnostic gap fully fixed. No ignored test or
weakened assertion hides it. A later three-line borrow-only cleanup passes the
loop layout by reference; it was reviewed statically and awaits targeted Clippy.
The verified functional source remains the frozen `eb15be082` run above.

The clean rebuild followed an ENOSPC failure during git rebase. Only the stopped
isolated optimiser target was removed; source, stash and logs were preserved.
The pre-ENOSPC baseline binary is `/tmp/lsp-2384-frozen-993a908ff.test`. No result
from an interrupted build is claimed. Baseline and candidate logs are
`/tmp/lsp-2384-retained-baseline.log` and `/tmp/lsp-2384-clean-candidate.log`.
The receipt is `/tmp/lsp-2384-evidence.json`; all 35 qualified/alias original
programs also have fresh C Tcl execution evidence in
`/tmp/lsp-2384-alias-qualified-native.json`.

The optional #2218 required-Jim test branch was type-checked by both successful
no-run builds. Its original three-test implementation proof remains the frozen
`c47d83bda` run; no equivalent #2218 runtime suite was repeated.
