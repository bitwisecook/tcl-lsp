# Expression fixtures

`exprs.txt` holds one expression per line; each was run as `puts [expr {EXPR}]` under tclsh 8.6/9.0, `tclvm --tcl-version` and the runtime's `run_script --quiet --tcl-version` (`engines.tsv`: file, release, expression, tclsh, VM, runtime), and as `set r [expr {EXPR}]; puts $r` through the optimiser and the analyser (`analyser.out`: optimiser rewrites, tcl9.0 and tcl8.6 diagnostic codes). `in-precedence.tcl` and `in-precedence-analyser.tcl` reproduce the original membership precedence bug. The corrected Tcl parser evaluates the latter procedure's condition as true.
