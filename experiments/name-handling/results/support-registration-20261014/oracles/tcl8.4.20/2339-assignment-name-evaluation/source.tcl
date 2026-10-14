proc p {} {set i 0; set a([incr i]) 5; if {$i == 0} {return zero} else {return nonzero}}
puts [p]
