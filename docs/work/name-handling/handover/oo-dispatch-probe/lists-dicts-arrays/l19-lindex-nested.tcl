set m {{1 2} {3 4}}
puts [list [lindex $m 1 0] [lindex $m {1 1}] [lindex $m end end] [lindex $m 5 0] [lindex $m] [lindex {} 0]]
