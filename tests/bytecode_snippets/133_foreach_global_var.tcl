proc test {} {
    foreach ::x {1 2 3} {
        set y $::x
    }
    return $::x
}
