proc test {} {
    set result {}
    foreach x {1 2 3 4 5 6 7 8 9 10} {
        if {$x == 3} continue
        if {$x == 7} break
        lappend result $x
    }
    return $result
}
