proc a {x} { b [info level] }
proc b {n} { return [list $n [info level] [info level 0] [info level -1] [lindex [info level 1] 0]] }
puts [a 7]
puts [info level]
puts [catch {info level 0} m]:$m
