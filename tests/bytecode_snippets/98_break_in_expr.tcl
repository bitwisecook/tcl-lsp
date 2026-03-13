proc test_break {} {
    set result {}
    while 1 {
        lappend result a
        lappend result [list b [break]]
        lappend result c
    }
    return $result
}
