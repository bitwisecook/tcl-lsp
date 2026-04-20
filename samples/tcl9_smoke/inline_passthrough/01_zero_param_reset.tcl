# F2 target: a zero-param uplevel-1 passthrough proc should inline
# into its caller.  Observable semantics must still match the pre-
# inlining behaviour — ``counter`` ends up at 0.
proc reset {} {
    uplevel 1 {set counter 0}
}
proc run {} {
    set counter 10
    reset
    return $counter
}
puts [run]
