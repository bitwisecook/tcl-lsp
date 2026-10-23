# Native double completion

`source.tcl` is executed unchanged by the six native engines named in
`manifest.json`. Each transcript contains ten counted outcomes. The manifest
records the script, executable and output hashes and the process exit status.

The controls cover numeric success, rejected input, infinity, argument count,
argument Return/Error settlement, C variable-read errors and replacement of the
modern C command-table function. C 8.4 and Jim retain fixed-table dispatch;
Jim has no variable-trace command in this interpreter, recorded explicitly as
`read_trace_unavailable`. Infinity is rejected by C 8.4 and accepted by later
C engines and Jim.

The C scalar protocol follows `ExprDoubleFunc` in C 8.4 `tclExecute.c` and
C 8.5–9.1 `tclBasic.c`: one operand, native conversion, Double on successful
completion, and OK/Error after argument execution. Operand object callbacks
and command observers are independent obligations. Jim's `JimExprOpNumUnary`
has separate Wide/Double/Boolean conversion paths; these transcripts do not
issue the C scalar protocol for that implementation.
