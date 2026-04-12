namespace eval foo {
    variable x 42
    proc bar {} {
        variable x
        return $x
    }
}
