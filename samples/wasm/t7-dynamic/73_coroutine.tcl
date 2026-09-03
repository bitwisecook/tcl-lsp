# T7: coroutines suspend a frame mid-body - requires a resumable Tcl frame.
proc gen {n} {
    for {set i 0} {$i < $n} {incr i} { yield $i }
    return done
}
coroutine g gen 3
puts [g]
puts [g]
puts [g]
puts [catch {g} r]
puts $r
puts [info commands g]
