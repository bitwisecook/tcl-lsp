set x global
proc p {} { global x; return $x ;# READ
}
puts [p]
