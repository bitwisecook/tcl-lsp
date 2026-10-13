# Authored simulator mock publication

Registry `SimulatorMockName` supplies the suffix and decision-log category/action
for the simulator's ASCII authored labels. Generator and pure-function consumers
use that owner; Tcl consumes an exact-label table. Nested labels share the first
namespace/final action convention. Unlisted labels and unsupported separator,
Unicode or derived-leading-colon forms decline without a generic name fallback.
This convention supplies no F5 binding, Native frame or appliance conformance.

`registration.tcl` retains the original Tcl helper and the exact new helper over
an owner-generated fixture. C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim
0.84-9-g5bac7c9 each report11 rows:6 supported labels return the expected owned
mock and resolve to the actual published command,5 unsupported/unlisted labels
return no mock. Rooted/nested labels, intentional suffix collisions, lone-colon
data and unsupported separator/Unicode forms are explicit. Original executables
remain unchanged. Matching original Tcl init libraries are used; no Tcltest is
used. Raw reports and executable/source hashes are retained.

`incorrect-info-frame-wrapper/` is an unaccepted first harness attempt: its
procedure wrapper evaluated info at the global frame, losing the linked mock
array. The corrected wrapper evaluates in its caller's frame. The original
reports remain separate; zero process exit status alone does not establish a
supported-label result.

One standalone Rust test passes the pure owner vectors against an existing
same-checkout Syntax artifact. Full Registry/xtask/VM acceptance remains pending.
The checked-in `_mock_names.tcl` currently projects the authored manual roster;
`cargo xtask gen-irule-test-data` must regenerate the complete selected catalogue
union and its check must pass before generator acceptance. The original manual
simulation roster remains hand-maintained and independent of environment
availability; the generator does not promote it to measured F5 command presence.

Replay the retained Tcl program with each matching original interpreter. Regenerate
its fixture through the shared owner, then compare exact row outputs. The complete
harness boot/pure-function controls and generated drift gate are separate checks.
