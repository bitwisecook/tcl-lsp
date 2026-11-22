foreach v {1..2 a1 1.2c3 {} -1 1.2a 1.2ab3 1_2} { lappend r [catch {package vcompare $v 1} m] }
puts $r
