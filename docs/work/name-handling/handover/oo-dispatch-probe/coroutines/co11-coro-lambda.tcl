# A coroutine over apply, and the creating command's arguments.
puts [coroutine g apply {{a b} { yield [expr {$a + $b}]; return [list [info level] [info coroutine]] }} 2 3]
puts [g]
puts [list [catch {coroutine} m] $m]
puts [list [catch {coroutine x} m] $m]
