# Exact-name catalogue API control

Question: after the MCP consumer forwards its actual retained analysis and
queries current shared metadata owners, does `command_info` preserve the
literal incoming command name?

The [unchanged fixed control](control-source.rs.txt) passes: the real
`HTTP::uri` row is found; all seven adverse values remain exactly as
supplied and are not found. This includes padded names, a tab/newline,
embedded NUL and a missing key. The control uses public MCP tool dispatch.

The selected `tcl-mcp` binary test target compiled successfully offline and
executed exactly one test, with one pass, zero failures and 132 filtered
tests. [Validation](validation.json), [exact invocation](offline.invocation.json),
[raw stdout](offline.stdout) and [raw stderr](offline.stderr) retain the
observed result. Total invocation time was 29.04 seconds; Cargo reports
26.56 seconds for compilation and the test harness reports 2.39 seconds.

The compiled test executable's SHA256 is
`363856c3e8169e3d2720507b7bd11959c69dca83ffe18fa9d5e40e8be9986447`.
[Binary provenance](binary.json) retains its exact Cargo artifact identity.
The 39-package conservative local manifest closure contains 5,307 frozen
source/header/manifests/configuration files, with zero changes during this
capture. Cargo emitted 361 artifact records, including the selected target
and 39 workspace packages. Artifact target-source hashes come from that
frozen snapshot; arbitrary sibling Rust files are not claimed compiled.
The closure includes VM and excludes standalone Runtime.

The earlier [five-error build](../validation-01/validation.json) and its
source/test failure evidence remain unchanged. The successful build uses
the same control source and fixed inputs. It forwards the existing analysis
to event readers, imports the same Tcl9.0+ availability constant from its
present owner, and reads variable-write pattern hints through the retained
source schema/context, actual arity/roles and original literal operands.
Those hints remain conditional; unknown advice proves no absence of writes.

Three existing compiler warnings are retained in [diagnostics](diagnostics.json).
No event, writer-hint or other MCP controls ran here. This catalogue API
result supplies no Tcl interpreter oracle result, native handler identity,
cell effect or appliance conformance claim.
