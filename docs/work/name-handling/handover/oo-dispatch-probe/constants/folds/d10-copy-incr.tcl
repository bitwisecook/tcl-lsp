proc p {} { set x 3; set y $x; incr y; return [list $x $y] }
puts [p]
