# Source words and normalized compiled words

This experiment records original written-source behavior in C Tcl 8.4.20,
8.5.19, 8.6.18, 9.0.4 and 9.1.0, and JimTcl commit
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. Each capture retains the exact
provider executable hash, input, raw output and exit status. C captures used
the matching original `init.tcl`; copies are retained under `original-init/`.
No tcltest package was used.

The quoted written word `$(1+2)` reads element `1+2` of the empty-root array
in C Tcl: the missing element raises an error, and a bound element returns
`ARRAY`. Jim evaluates the arithmetic and returns `3`. The braced word
`{$(1+2)}` remains literal for all six providers. Jim does not support the
queried `info library` operation; its raw response is retained.

`first-premise-failure/` preserves the original probe and its C failures: the
initial expectation that C would return literal text was wrong.
`corrected-source-observation/` records the missing and bound array controls.
The compiler/VM normalized compiled-word protocol is a separate representation;
these source observations do not grant native admission or physical identity.
F5 and hosted interpreters were not exercised.
