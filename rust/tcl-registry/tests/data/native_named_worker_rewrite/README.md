# Native named worker rewriting

`probe.tcl` renames the installed `info` ensemble and its `locals` worker,
then configures the same public ensemble with a literal member map. The
observations come from C Tcl 8.4.20–9.1.0 and current Jim Tcl.

C Tcl 8.6–9.1 compiles the accepted worker form as a private-name invocation.
When that worker compiler declines the argument count, the native compiler
retains all four public invocation words, pushes the selected private name,
and emits `invokeReplace 4 2`. The canonical member replaces the original
selector; it does not remove an operand from the public rewrite vector.
C Tcl 8.5 retains ordinary public invocation. Unsupported surfaces retain
their actual diagnostic.

The original worker token, public configuration and original parser operands
provide selection authority. Reported full names are literal output only.
