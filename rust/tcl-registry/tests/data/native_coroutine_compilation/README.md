# Original coroutine compilation

The disassembly tables contain unchanged procedure bodies compiled by C Tcl
8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, plus current Jim Tcl observations.
`probe.tcl` captures compiler availability, the exact source bytes, original
operand chronology and disassembly. `manifest.json` identifies the engines and
raw output hashes. Memory addresses in disassembly are observations.

The `runtime-*.tsv` tables contain thirteen independent original scripts per
engine. Columns are case, original source hex, completion code and result hex.
They cover namespace and caller-frame routing, operand-time rename, empty and
expanded tailcalls, original List values, coroutine rename, illegal yield,
empty relay and namespace deletion. The runtime manifest records raw output
hashes. Jim's missing coroutine commands remain measured errors.

C8.6 and C9.0 evaluate the original tailcall head as a placeholder and replace
it with the executing procedure namespace at execution. C9.1 evaluates the
namespace first and registers a compile-known target with the command-literal
role. YieldTo retains a namespace-prefixed original List; a namespace-only List
still produces the native wrong-argument error before suspension. Compiler
preparation, original object ownership, suspension availability and handler
lookup are independent obligations.
