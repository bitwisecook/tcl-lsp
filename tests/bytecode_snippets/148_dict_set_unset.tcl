proc test {} {
    set d [dict create a 1 b 2]
    dict set d c 3
    dict set d a 10
    dict unset d b
    return $d
}
