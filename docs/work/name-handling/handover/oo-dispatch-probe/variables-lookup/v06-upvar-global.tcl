set g global
proc p {} { upvar #0 g alias; return $alias ;# READ
}
puts [p]
