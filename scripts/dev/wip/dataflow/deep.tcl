proc deep {} {uplevel 3 {::set x 2}}
proc w2 {} {set x w2; deep; return $x}
proc w1 {} {set x w1; set r [w2]; return "$r $x"}
proc outer {} {set x 1; set r [w1]; return "$r $x"}
puts [outer]
