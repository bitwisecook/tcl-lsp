The fixture contains 17 identical procedure bodies evaluated by Tcl 8.4.20,
8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84. `observations.json` retains each
exact source, source and interpreter SHA-256, process exit, stdout and stderr.
The Tcl sources query the native procedure disassembly when that API exists.

`windows.tsv` retains the body and completion bytes as hexadecimal. `inline`
identifies the C8.6+ native unset compiler's accepted bodies; its disassembly
contains no ordinary invoke instruction. `unset_count` counts the disassembled
unset instructions. Older C releases and Jim have no unset compiler.

The controls cover empty operands, scalar and array receivers, rooted names,
quiet and end-of-options flags, unrecognized option-shaped variable names,
native substituted-prefix validation, constant and dynamic expansion, and
sequential index callbacks. The ordered callback case fails on `a(FIRST)` and
leaves `::seen` equal to `FIRST`; the second index is never evaluated.
