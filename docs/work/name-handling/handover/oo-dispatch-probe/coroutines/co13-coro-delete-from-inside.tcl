# Deleting the running coroutine's own command from inside it.
proc body {} { yield 1; rename [info coroutine] {}; return after-delete }
coroutine c body
puts [list [catch c m] $m [info commands c]]
