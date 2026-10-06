proc p {} {
    set x OLD
    upvar 0 x y
    unset x
    set y NEW
    puts $x
    puts $y
}
p
