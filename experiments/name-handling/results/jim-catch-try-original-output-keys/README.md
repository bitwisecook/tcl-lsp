Original catch/try variable-name publication
==========================================

Question: do catch/try publish their actual original result/options name headers
as variable keys, preserve an earlier key on overwrite, and use the actual global
frame for the error-code reset while restoring a namespaced caller?

Captures ran original Jim revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`,
rechecked against upstream HEAD at execution on 2026-10-15 at 15:12, 15:14 and
15:18 UTC. All compiled and completed with exit 0 and no stderr. Each original namespace
suite passed 79/79 through its original testing.tcl and tcltest.tcl. C API probes
use no tcltest harness. Each manifest records all 247 unchanged provider files,
source/configuration/executable/compiler/linked-library hashes, inputs, invocations,
stdout/stderr and completion. Rebuildable probes are excluded; captured hashes
remain recorded and are checked if an executable is present.

The first capture records 18 output-name rows. The second adds nine global-reset
rows. The third adds failed output assignments for both commands, retaining all
previous observations as rows. Actual integer name headers are used as command
arguments, then compared with the actual original variable-table keys. Empty
message names create no variable; options names still publish. Overwriting the
pre-existing dictionary variable preserves its earlier table key, rather than
replacing it with the later original output-name header.

A result-name link to an element of an odd dictionary value fails its setter.
Both commands skip the untouched, stringless integer options-name header. Try
keeps the original body error code. Catch returns success after publishing the
original body's completion code as its result; it does not propagate the guest
setter failure. Native/Host refusals remain a separate unavailable execution.

The additional cases invoke the original Jim_SetGlobalVariableStr from a native
command reached inside an actual namespaced procedure. A new global and an
existing dictionary global reset successfully. A global link to an element of an
odd dictionary value fails. All three restore the exact caller frame and namespace
object, and preserve the exact same local errorCode value object.

`incorrect-array-control-f462.rs` retains the exact unexecuted Runtime control
beforeimage from checkpoint f462b0bc7. Its expectation that a Jim dictionary
global would reject scalar replacement was unsupported and incorrect. These
original observations correct that expectation to success and separately supply
the actual linked-element failure. No Runtime failure or original failure is
inferred from that unexecuted control. The original 174 retained controls remain
unchanged.

Runtime fixtures derive from capture03 stdout and exact C script literals.
`verify.py --current-provider` verifies the sealed package/current provider
without executing an interpreter. Rust controls remain unexecuted until Root
runs a fresh successor build. Physical native-reference-count parity beyond the
recorded header/type/getter/selection cells, and result value bytes, are outside
this experiment.
