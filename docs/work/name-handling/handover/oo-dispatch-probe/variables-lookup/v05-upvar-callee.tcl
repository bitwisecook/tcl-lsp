proc setit {name} { upvar 1 $name v; set v callee }
proc p {} { setit x; return $x ;# READ
}
puts [p]
