proc p {} {
    set x 4
    if {$x > 3} { set r big } else { set r small }
    return $r
}
puts [p]
