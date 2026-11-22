proc setit {name} { upvar 1 $name v; set v 5 }
proc p {} { setit x; return [expr {$x + 1}] }
puts [p]
