# Renaming a coroutine command changes info coroutine; renaming it to {} ends it.
proc body {} { while 1 { yield [info coroutine] } }
coroutine a body
rename a b
puts [b]
puts [list [info commands a] [info commands b]]
rename b {}
puts [list [info commands b] [catch b msg] $msg]
