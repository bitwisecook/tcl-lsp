# An error in a procedure a top-level for body calls, uncaught in a script
# file: the body and the for add their frames where the release runs it.
proc e {} { error boom }
puts start
for {set i 0} {$i < 1} {incr i} {
    e
}
puts unreached
