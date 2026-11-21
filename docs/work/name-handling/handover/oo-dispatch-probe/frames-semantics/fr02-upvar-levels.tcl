proc a {} { set v 1; b; return $v }
proc b {} { c }
proc c {} { upvar 2 v vv; incr vv }
puts [a]
proc d {} { set s 1; upvar 0 s t; incr t; return $s }
puts [d]
array set arr {k 1}
proc e {} { upvar #0 arr(k) el; incr el }
e
puts $arr(k)
