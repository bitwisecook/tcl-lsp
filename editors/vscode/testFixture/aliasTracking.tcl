proc fixed {} {
    upvar 1 caller_x alias_x
    set alias_x 5
    incr alias_x
    return $alias_x
}
set caller_x 0
fixed
