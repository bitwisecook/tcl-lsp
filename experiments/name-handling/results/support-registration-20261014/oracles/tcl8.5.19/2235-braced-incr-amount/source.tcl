proc p {} {set n 3; set x 1; incr x {$n}; return $x}
puts [catch {p} m]
puts $m
