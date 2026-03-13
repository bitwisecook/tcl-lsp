proc test_types {s} {
    set a [string is alpha $s]
    set b [string is digit $s]
    set c [string is alnum $s]
    list $a $b $c
}
