# Entered child completion controls

`probe.tcl` runs the same eight ASCII scripts in each real interpreter. The
TSV columns are case number, original source bytes in hexadecimal, completion
code and result bytes in hexadecimal. `manifest.json` records the interpreter
versions, executable hashes and current Jim source commit.

Run `tclsh probe.tcl` or `jimsh probe.tcl` with the selected interpreter to
reproduce its table. Each case is caught by the probe; the error case contains
an additional inner catch, so its outer completion remains successful.

The Rust comparison creates each actual core and its independently selected
source compiler. Jim's `binary` observer is installed as a distribution
extension whose Tcl wrapper tailcalls its synchronous native member. The
scripts exercise ordinary command substitution, catch, subst, expression,
conditional and loop continuations, including successful and error results.
Compiler-mutation tests exercise the host provenance contract independently
of these interpreter observations.
