proc p {} {
    set x 5
    set y [expr {$x * 2}]
    set z [expr {$y + 1}]
    return $z
}
puts [p]
