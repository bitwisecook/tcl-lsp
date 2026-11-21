proc gen {} { yield; set i 0; while 1 { yield [incr i] } }
coroutine g gen
puts [g][g]
puts [catch {g 1 2} m]:$m ;# BAD coroutine takes one arg
