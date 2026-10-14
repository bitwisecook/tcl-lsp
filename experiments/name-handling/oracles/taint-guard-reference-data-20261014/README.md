# Guard reference data

`guard-reference-data.tcl` measures variable-reference spelling inside `string`
operands. C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and the retained JimTcl
executable all observe distinct literal sigil and empty scalar names. A braced
literal `$literal` remains data. Array element reads work independently of
scalar names; these observations do not prove that guarding one element bounds
an entire array.

The `CLOSER` result is 0 for C Tcl 8.4–8.6 and Jim, and 1 for C Tcl 9.0–9.1,
under the deliberately distinct `a{b` and `a{b}c` values. This pins the actual
release's closing rule.

`results.json` retains executable, matching C `init.tcl` and available provider
library hashes before and after each direct run, with stdout and stderr. No
`tcltest` is used. The Rust consumer selectors are separate checks; this capture
makes no Rust, F5, command implementation or physical variable binding claim.
