proc compute {n} {
    set result 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i % 2 == 0} {
            incr result $i
        } else {
            incr result [expr {-$i}]
        }
    }
    return $result
}
