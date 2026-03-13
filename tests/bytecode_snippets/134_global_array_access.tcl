proc test {} {
    set ::a(1) 1
    set ::a(2) $::a(1)
    return $::a($::a(1))
}
