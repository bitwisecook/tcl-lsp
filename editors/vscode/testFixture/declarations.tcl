# Fixture exercising global/variable/upvar declarations.
proc with_global {} {
    global shared_var
    set shared_var 42
    return $shared_var
}

namespace eval mymod {
    variable counter 0
    proc bump {} {
        variable counter
        incr counter
    }
}

proc with_upvar {name} {
    upvar 1 $name local_copy
    return $local_copy
}
