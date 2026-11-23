# tailcall inside a coroutine body replaces the coroutine's base frame; the command stays.
proc helper {} { yield h1; return h-done }
proc body {} { yield b1; tailcall helper }
puts [coroutine c body]
puts [c]
puts [c]
puts [info commands c]
