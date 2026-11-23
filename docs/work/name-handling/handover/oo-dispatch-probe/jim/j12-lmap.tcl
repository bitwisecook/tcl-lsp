puts [lmap x {1 2 3} {expr {$x * $x}}]
puts [lmap x {1 2 3} {if {$x == 2} continue; set x}]
