# Actual SamePosition first-cause handoff diagnostic

The selected VM lib-test build succeeded with explicit `tmp/name-target-issues`
and `-j2`. Its native list observed the exact original SamePosition test once;
that one control ran once and failed at the unchanged `first(vm)` accessor.
One test failed, none passed or were ignored;1197 were filtered out. No retry or
other control ran.

The full original cause is
`ValueAccessRefusal(CommandProtocolUnavailable("original OO cleanup frame description"))`.
The actual gated observations establish its handoff:

| Actual boundary | Activation / host / entered | Cause and storage |
| --- | --- | --- |
| Before outer invocation |0 /0 / false | No stored cause |
| Nested public settlement and return |0 /1 / true | Same complete cause returned and retained in storage |
| Outer public settlement |0 /0 / false | Identical complete cause returned; existing clear branch executes |
| Before original equality assertion |0 /0 / false | Owned cause remains identical; stored cause is None |

The original accessor then panics with `full entered first cause`. This measures
an incompatible fixture expectation after intentional outer-boundary retirement;
it supplies no production handoff bug. Assertions after that panic remain
unreached, including final callback count, stored Some and cleanup assertions.
No assertion, callback flow, arguments, fixture/profile, currency predicate or
clone/clear/return branch changed. Temporary logging creates no Native entry,
provider or additional scope; Ok logging emits only Code.

[Full raw stderr](control.stderr), [stdout](control.stdout), [exact list](control-list.stdout),
[execution](execution.json) and [summary](summary.json) retain complete causes,
actual predicates, original panic, timestamps, argv and stable binary hashes.
[Logging-only patch](instrumentation.patch), two compressed original owner
images and [runner](run.py) retain the temporary source context. The original
question15 and prior failures remain unchanged; this is its separate
SamePosition neighbour, not a forward or procedure rerun.

The conservative hash envelope covers23,357 tracked files from15 authored path
packages plus root build configuration. All recorded files stayed unchanged
through the build/control apart from the two authorised diagnostic preimages.
[Actual Cargo graph](artifact-graph.json) contains70 emitted records,65 fresh,
and15 local package manifests. The [VM rustc dep-info](vm-rustc-depinfo.d) lists
1512 selected-target inputs:1421 were in the preflight envelope,91 shared
fixtures were outside it and lack independent before-build attestation.
[Membership limits](source-membership-summary.json) distinguish these scopes.
The manifest envelope is not a claim that every recorded file compiled, and the
VM dep-info is not complete transitive binary provenance. Only two full original
owner images plus the patch were retained; no fixture-tree archive or duplicate
binary was created. Standalone Runtime Clock/fs owners are outside this target.

Finally restored both owners byte-exact and released their source/cache leases.
Original-to-restored inventory has zero changes. The diagnostic executable is
noncurrent after restoration and must not be reused. No fixture correction,
production repair, wider suite, C oracle or proof renewal follows this capture.
