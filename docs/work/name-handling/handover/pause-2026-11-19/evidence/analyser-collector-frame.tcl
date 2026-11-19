set globalValue shared
proc check {arg} {set local $arg; uplevel #0 {set globalSeen $globalValue}; return $local}
set first [check first]
set second [check second]
puts [list $first $second $globalSeen]
