# Frame of a coroutine body: level, namespace, and persistence of locals across yields.
namespace eval ns {
    proc body {} {
        set local 1
        yield [list [info level] [namespace current] [info level 0]]
        incr local
        yield [list [info level] $local]
        return [info exists ::outer]
    }
}
proc driver {} { coroutine c ns::body }
set outer 1
puts [driver]
puts [c]
puts [c]
