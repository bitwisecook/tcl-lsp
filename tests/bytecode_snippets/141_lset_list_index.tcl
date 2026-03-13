proc test {} {
    set x {{1 2} {3 4}}
    lset x {1 1} 5
    return $x
}
