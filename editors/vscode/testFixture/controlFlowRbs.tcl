# FP-RBS control-flow family (PR #634) — only the empty-foreach read fires.
proc silent_tailcall {cond} {
    if {$cond} { tailcall g } else { set r 1 }
    return $r
}
proc silent_foreach {} {
    foreach x {1 2 3} { set y $x }
    puts $y
}
proc silent_while1 {} {
    while 1 { set w 1; break }
    puts $w
}
proc fires_empty_foreach {} {
    foreach x {} { set y $x }
    puts $y
}
# FP-RBS-19 (#756) — a may-run loop whose body defines the variable is assumed
# to run, so an after-loop read is silent (matches C Tcl).
proc silent_dynamic_foreach_accumulator {items} {
    foreach v $items { lappend acc $v }
    return $acc
}
proc silent_dynamic_while {n} {
    while {$n > 0} { set y 1; incr n -1 }
    puts $y
}
