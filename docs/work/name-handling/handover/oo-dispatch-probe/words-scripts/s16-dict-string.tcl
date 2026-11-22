set d [dict create a 1 b {x y}]
puts [list $d [dict get $d b] [llength $d] [catch {dict get {a 1 b} a} m] $m]
