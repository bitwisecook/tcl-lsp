set d {b 2 a 1 b 3}
puts [list [dict get $d b] [dict size $d] [dict keys $d] $d [dict create b 2 a 1 b 3] [dict set d c 4] [dict unset d a] [dict exists $d b] [catch {dict get $d zz} m] $m]
