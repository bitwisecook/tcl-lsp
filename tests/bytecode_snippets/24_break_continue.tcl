for {set i 0} {$i < 10} {incr i} {
    if {$i == 3} continue
    if {$i == 7} break
    set x $i
}
