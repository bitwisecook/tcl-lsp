proc countdown {n} {
    set result {}
    while {$n > 0} {
        lappend result $n
        incr n -1
    }
    return $result
}
