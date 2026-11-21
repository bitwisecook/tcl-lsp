set x global
proc p {} { set x local; return $x ;# READ
}
puts [p]
