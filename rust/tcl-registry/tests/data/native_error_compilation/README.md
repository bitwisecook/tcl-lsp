# Native Error compilation

`probe.tcl` records original Error operands, native compiler instruction order,
local declaration order and guest completion for C Tcl 8.4, 8.5, 8.6, 9.0, 9.1
and Jim 0.84. Each TSV preserves the observed result bytes and normalises the
disassembly to opcode names; releases without the disassembly command have an
empty opcode column.

C 8.4 and 8.5 use ordinary invocation. C 8.6 and 9.0 build the optional error
information/code as an ordered List. C 9.1 starts with the empty options literal
and inserts each original option through `dictPut`. Accepted native Error
compilation completes through `returnImm` with error code one and level zero.
Literal parser expansion keeps the same original member geometry.
