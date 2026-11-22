proc p {} { set x 1; set n x; set $n 2; return $x }
puts [p]
