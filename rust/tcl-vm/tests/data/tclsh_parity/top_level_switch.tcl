# An error in a procedure a top-level switch arm calls, uncaught in a script
# file: the arm and the switch add their frames where the release runs it.
proc e {} { error boom }
puts start
switch -- a {
    b {}
    a {
        e
    }
}
puts unreached
