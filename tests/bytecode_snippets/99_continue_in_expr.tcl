proc test_continue {} {
    set result {}
    set i 0
    while {[incr i] <= 10} {
        lappend result $i
        lappend result [list b [continue] c]
        lappend result c
    }
    return $result
}
