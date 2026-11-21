set gv 9
proc p {} { variable gv; return $gv }
puts [p]
