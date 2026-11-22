source lib/util.tcl
proc compute {n} {
    set doubled [::util::scale $n 2]
    set tripled [::util::scale $n 3]
    return [list $doubled $tripled]
}
puts [compute 5]
