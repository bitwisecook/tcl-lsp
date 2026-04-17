proc test {} {
    lappend x(0) a b c
    lappend x(1) d e
    return [list $x(0) $x(1)]
}
