# A coroutine that returns ends its command; a coroutine that errors does too.
proc once {} { yield a; return b }
puts [coroutine c once]
puts [c]
puts [list [catch c m] $m]
proc bad {} { yield ok; error boom }
puts [coroutine d bad]
puts [list [catch d m] $m [info commands d]]
