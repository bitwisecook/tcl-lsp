set s 1
puts [list [catch {set s(x) 2} m] $m [catch {array set s {a 1}} m2] $m2]
array set a2 {x 1}
puts [list [catch {set a2 5} m3] $m3]
