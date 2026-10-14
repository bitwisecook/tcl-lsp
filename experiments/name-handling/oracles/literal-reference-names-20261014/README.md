# Literal reference names

The original `probe.tcl` runs directly under C Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4, 9.1.0 and JimTcl. `results.json` records executable hashes, exact
commands, matching original C library paths and hashes, output hashes and exit
codes. The matching original C `init.tcl` files are retained unchanged under
`original-libraries/`. The probe does not use tcltest. Jim was built from upstream
commit `5bac7c99ad65864c87da513e22e2f01703fa4e03`.

All six providers return the exact literal sigil name, empty scalar name and
array element values. C Tcl 9.0/9.1 read `${{x}}` as scalar name `{x}`; C Tcl
8.4/8.5/8.6 and Jim close at the first `}`, read name `{x`, and append the
remaining literal `}`. `x`, `{x}` and `{x` have distinct setup values so a
normalised-name collision is visible. Neither array spelling reads its root
scalar value.

The compiler's scalar constant substitution consumer therefore queries the
shared scalar-reference owner using the original whole token and selected lexer
configuration. Array references decline that scalar path, and compound words
retain their separate evaluation obligations. The Rust tests retain all prior
assertions and add these exact source shapes with an unrelated root constant as
a negative control. These provider observations do not establish Rust test
acceptance or Native dispatch/frame eligibility. F5 remains unanswered.
