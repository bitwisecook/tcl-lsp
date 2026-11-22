proc p {} {
    set l {a b c}
    set d {k 1 j 2}
    return [list [lindex $l 1] [llength $l] [dict get $d j] [lsort {c a b}]]
}
puts [p]
