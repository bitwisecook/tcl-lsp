proc test {} {
    set sum 0
    for {set i 0; set j 10} {$i < $j} {incr i; incr j -1} {
        incr sum $i
    }
    return $sum
}
