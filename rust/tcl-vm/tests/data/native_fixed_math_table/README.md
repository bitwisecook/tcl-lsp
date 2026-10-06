# Native fixed math-function table

`probe.tcl` records zero-, one- and two-argument expression calls, argument
evaluation on an arity failure or missing function, and a same-spelled
`::tcl::mathfunc::abs` procedure. C Tcl 8.4 resolves its fixed function table
before evaluating arguments and ignores that procedure; C Tcl 8.5–9.1
resolves the ordinary math-function command after argument evaluation. Current
Jim uses its own fixed function surface and integer result formatting.

`probe.c` registers an actual two-argument function with `Tcl_CreateMathFunc`
on C Tcl 8.4–8.6. The records include original function execution, an arity
failure with a substituted argument, command replacement, and fixed-table
replacement during a substituted argument. C Tcl 8.4 retains its entered
builtin handler across that replacement and selects the new function on the
next compilation. That API is
absent from the C Tcl 9 headers; the C Tcl 9 records use the actual command
surface in `probe.tcl`.

`manifest.json` identifies original source, engines, headers and static
libraries by SHA-256 and retains exact process arguments and exit status.
Each process has a 60-second budget.

The portable `CALL_FUNC1` instruction retains the exact
`NativeMathFunctionBinding` outside the value stack. Its immediate counts
value operands only; `FunctionAsm` retains the complete original
`NativeMathFunctionPrerequisite`. The VM validates both before admission and
retains the actual selected builtin handler before argument evaluation,
preserving original value objects and the entered handler across table
replacement. A later admission still validates the complete current table. This
portable layout covers fixed builtin calls; it is independent of C Tcl's
`callBuiltinFunc1` index and custom `callFunc1` name-on-stack layout.
