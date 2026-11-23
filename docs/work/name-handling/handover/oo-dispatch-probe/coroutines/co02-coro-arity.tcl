# Arity of a coroutine command: zero or one argument (yield), any number (yieldto).
proc g {} { set got [yield start]; set more [yieldto string cat]; return [list $got $more] }
puts [coroutine c g]
puts [catch {c a b} msg]:$msg
puts [c one]
puts [c x y z]
puts [info commands c]
