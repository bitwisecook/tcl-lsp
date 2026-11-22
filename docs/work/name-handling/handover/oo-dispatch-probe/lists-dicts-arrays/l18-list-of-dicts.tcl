set recs {{name a age 3} {name b age 1}}
puts [lsort -command {apply {{x y} {expr {[dict get $x age] - [dict get $y age]}}}} $recs]
