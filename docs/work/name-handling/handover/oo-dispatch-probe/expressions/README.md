# Expression fixtures

`exprs.txt` holds one expression per line; each was run as `puts [expr {EXPR}]` under tclsh 8.6/9.0, `tclvm --tcl-version` and the runtime's `run_script --quiet --tcl-version` (`engines.tsv`: file, release, expression, tclsh, VM, runtime), and as `set r [expr {EXPR}]; puts $r` through the optimiser and the analyser (`analyser.out`: optimiser rewrites, tcl9.0 and tcl8.6 diagnostic codes). `in-precedence.tcl` shows the `in`/`ni` precedence divergence; `in-precedence-analyser.tcl` is the procedure for which the analyser reports a false I230.
