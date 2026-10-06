# Native namespace-binding compilation

`probe.c` inspects the actual command compileProc, emitted VARIABLE/NSUPVAR
instructions and counted procedure local-name table for the twenty fixed
procedure bodies. The five TSV files contain C Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0 results. A count of -1 means the original procedure body has
no bytecode representation after its syntax error.

The controls distinguish parser literal expansion, release-specific final
TEXT tail acceptance, partial local declarations retained after a declined
compiler, and interleaved namespace binding and value compilation. Command
substitutions can independently add locals; the binding owner does not claim
those declarations. `provenance.json` identifies the exact native binary,
headers, compiler/parser source, static library and output bytes.
