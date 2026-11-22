set d {a {x 1} b 2}
dict set d a y 2
puts [list $d [dict get $d a y] [dict merge {a 1 b 2} {b 3 c 4}] [dict filter {a 1 b 2 c 3} key {[ab]}] [dict filter {a 1 b 2} value 2] [dict map {k v} {a 1 b 2} {expr {$v*10}}] [dict incr d b 5] [dict lappend d c x y]]
