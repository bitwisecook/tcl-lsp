proc p {} {
    set x 1
    set z 9
    upvar 0 x y
    set first [expr {$y + 1}]
    upvar 0 z y
    set second [expr {$y + 1}]
    puts $first
    puts $second
}
p
