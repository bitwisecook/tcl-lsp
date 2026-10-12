namespace eval ::nh {
    proc emit {value args} {list local $value $args}
    variable prefix [namespace code [list emit {a b}]]
}
proc emit args {return global}
eval [linsert $::nh::prefix end {c d}]
