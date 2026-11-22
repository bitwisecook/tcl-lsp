foreach {a b} {1.2 1.10 1.2a1 1.2 1.2b1 1.2a2 1.2 1.2.0 01.2 1.2 8.6 8.6.18 1.0 1} { lappend r [package vcompare $a $b] }
puts $r
