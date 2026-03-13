proc test {} {
    set a(1) one
    set a(2) two
    list [array exists a] [array names a] [array size a]
}
