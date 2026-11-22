set l {a {b c} d}
puts [list [llength [list {*}$l e]] [llength [list {*}"x y" z]] [llength [list {*}{} q]]]
