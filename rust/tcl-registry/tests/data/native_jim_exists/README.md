The pinned original Jim core supplies these public `exists` results. `receipt.json`
retains the source, executable, input and output digests. `driver.tcl` runs the same
`exists.tcl` statements and prints their final joined value; Runtime tests compare
that value against `stdout`. No `tcltest` package or replacement helper is used.

The cases cover original option abbreviations and errors, NUL-bearing exact enum
lookup, command kinds, relative namespace fallback, scalar/empty/dictionary names,
and linked variables. The Runtime separately refuses an existing command's
channel classification when its actual Jim channel flag has no retained owner;
these observations establish no positive native channel classification.
