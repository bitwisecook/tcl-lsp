# Precision regression fixture: every variable below IS read, so no dead-store /
# unused / read-before-set diagnostic (W210/W211/W214/W220) should fire on them.
# (Verified against C tclsh semantics: return reads its value, a write trace
# fires on `set`, an eval braced body runs in the current scope, and vars inside
# [expr {...}] command substitutions are read.)

proc p_return {} { set x 1; return $x }
proc p_expr_sub {n} { set scale 3; return [expr {$n * $scale}] }
proc p_incr_expr {} { set w 5; set i 0; incr i [expr {$w}]; return $i }
proc p_eval_body {} { set x 1; eval {puts $x} }
proc p_traced {} { trace add variable x write cb; set x 1 }

# Anchor: a non-lifecycle diagnostic (W100 unbraced expr) so the test can
# observe that analysis ran on this document.  `a`/`b` are set and `y` is read
# here so the anchor itself contributes no lifecycle diagnostic (otherwise the
# unset `$a`/`$b` would fire W210 and the unused `y` would fire W211/W220,
# defeating the "no false lifecycle diagnostics" assertion).
set a 1
set b 2
set y [expr $a + $b]
puts $y
