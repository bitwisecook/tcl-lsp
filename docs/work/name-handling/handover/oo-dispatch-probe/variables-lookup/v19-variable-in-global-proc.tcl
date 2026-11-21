set gv globalvar
proc p {} { variable gv; return $gv ;# READ
}
puts [p]
