# eval of a braced literal — compiler sees through the barrier
proc run {} {
    set x 5
    eval {incr x}
    return $x
}
puts [run]
