proc incr {args} {return 0}
proc bump {name} {upvar 1 $name v; set v [expr {$v + 1}]}
proc user {} {set x 0; incr x; return $x}
proc c {} {set x 0; bump x; return $x}
puts "[c] [user]"
