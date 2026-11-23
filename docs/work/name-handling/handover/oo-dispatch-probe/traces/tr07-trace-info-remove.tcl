# trace info variable lists registrations newest first; remove deletes one.
proc cb {args} {}
proc cb2 {args} {}
set x 1
trace add variable x {read write} cb
trace add variable x unset cb2
puts [trace info variable x]
trace remove variable x {read write} cb
puts [trace info variable x]
puts [trace info variable nosuch]
