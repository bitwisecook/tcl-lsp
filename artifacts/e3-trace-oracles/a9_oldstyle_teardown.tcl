# P2-1 oracle: does an 8.x legacy-installed unset trace still get the `rwua`
# LETTER when it is fired by namespace teardown rather than an explicit unset?
set ::log {}
proc rec {label n1 n2 op} { lappend ::log "$label:$op" }

# Legacy (old-style) registration, fired by `namespace delete`.
namespace eval ::legacy {
    variable x 1
    trace variable x u {rec L}
}
namespace delete ::legacy
puts "legacy-teardown: $::log"

# Modern registration, same teardown path, for contrast.
set ::log {}
namespace eval ::modern {
    variable y 1
    trace add variable y unset {rec M}
}
namespace delete ::modern
puts "modern-teardown: $::log"

# Both on one variable, interleaved, through teardown.
set ::log {}
namespace eval ::both {
    variable z 1
    trace variable z u {rec Lz}
    trace add variable z unset {rec Mz}
}
namespace delete ::both
puts "both-teardown: $::log"

# Control: the same legacy trace fired by an explicit unset.
set ::log {}
namespace eval ::expl {
    variable w 1
    trace variable w u {rec E}
}
unset ::expl::w
puts "legacy-explicit: $::log"
namespace delete ::expl

# And a legacy trace on an array element through teardown.
set ::log {}
namespace eval ::arr {
    variable a
    set a(k) 1
    trace variable a(k) u {rec A}
}
namespace delete ::arr
puts "legacy-elem-teardown: $::log"
