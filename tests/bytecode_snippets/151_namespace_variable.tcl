namespace eval myns {
    variable count 0
    variable name "test"
    proc inc {} {
        variable count
        incr count
    }
}
