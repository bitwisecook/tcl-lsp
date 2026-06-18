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
