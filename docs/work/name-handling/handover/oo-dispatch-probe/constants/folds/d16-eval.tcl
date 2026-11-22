proc p {} { set x 1; eval {set x 2}; return $x }
puts [p]
