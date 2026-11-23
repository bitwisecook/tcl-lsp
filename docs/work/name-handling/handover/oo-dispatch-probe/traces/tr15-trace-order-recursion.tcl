# Several traces fire newest first; a write inside a write trace does not re-fire it.
proc a {n1 n2 op} { puts "a"; upvar 1 $n1 v; set v [expr {$v + 1}] }
proc b {n1 n2 op} { puts "b" }
set x 1
trace add variable x write a
trace add variable x write b
set x 10
puts "x=$x"
