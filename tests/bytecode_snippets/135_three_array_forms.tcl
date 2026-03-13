proc test {} {
    global a
    set a(1) 42
    list ${a(1)} $a(1)
}
