# T2: foreach in its common shapes: one var, two vars, two lists, nested.
set total 0
foreach n {1 2 3 4} { incr total $n }
puts $total
foreach {k v} {a 1 b 2 c 3} { puts "$k=$v" }
foreach x {1 2} y {a b} { puts "$x$y" }
set pairs {}
foreach i {1 2} { foreach j {x y} { lappend pairs $i$j } }
puts $pairs
puts [lmap n {1 2 3} {expr {$n * $n}}]
