proc p {} {
    set x X
    set y Y
    upvar 0 x z
    upvar 0 y z
    set z NEW
    puts $x
    puts $y
}
p
