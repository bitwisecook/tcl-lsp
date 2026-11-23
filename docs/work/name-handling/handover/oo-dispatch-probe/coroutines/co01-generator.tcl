# Generator idiom: an infinite loop that suspends at each yield.
proc counter {start} {
    yield [info coroutine]
    set i $start
    while 1 {
        yield $i
        incr i
    }
}
puts [coroutine c counter 5]
puts [list [c] [c] [c]]
rename c {}
puts [info commands c]
