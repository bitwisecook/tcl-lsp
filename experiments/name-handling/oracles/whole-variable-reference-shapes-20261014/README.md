# Whole variable-reference shapes

The source probe distinguishes complete scalar references from compound words
and array references in C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, and
JimTcl commit `5bac7c99ad65864c87da513e22e2f01703fa4e03`. Captures retain the
original input, raw stdout/stderr, exit status, executable SHA-256 and the
matching original C `init.tcl` hashes and files. No tcltest package was used.

`$a:b` returns `A:b` even when variable `a:b` contains `AB`: an isolated colon
ends the unbraced variable name. `$café` returns `PREFIXé` in C and `UNICODE`
in Jim. `${}` reads the empty scalar name. `${scalar(open}` reads its complete
literal scalar name. `$arr(key)` and `${arr(key)}` both read the array element.
`${a{b}c}` reads `a{b}c` in C Tcl 9.0/9.1; C 8.4–8.6 and Jim close at the
first brace and append the remaining literal `c}`.

The initial probe used an invalid braced Tcl word for the setup variable name.
`first-invalid-fixture/` retains that exact fixture and every provider failure.
`corrected-source-observation/` quotes setup names and every provider exits
successfully. These observations ground syntax classification only; they do
not prove any Rust implementation result, current cell identity, successful
read under another state, Native admission or F5/hosted behavior.
