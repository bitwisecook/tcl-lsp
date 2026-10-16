# An error in a procedure a top-level while body calls, uncaught: a script
# file stops there, and stdin reads on to the if after it.
proc e {} { error boom }
puts start
set i 0
while {$i < 1} {
    incr i
    e
}
puts after
if {$i} {
    e
} else {
}
puts end
