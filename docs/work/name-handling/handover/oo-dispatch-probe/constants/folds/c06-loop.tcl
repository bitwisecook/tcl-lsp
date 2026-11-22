proc p {} {
    set s 0
    for {set i 0} {$i < 5} {incr i} { incr s $i }
    return $s
}
puts [p]
