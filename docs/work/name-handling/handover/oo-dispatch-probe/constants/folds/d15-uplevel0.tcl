proc p {} { set x 1; uplevel 0 {set x 2}; return $x }
puts [p]
