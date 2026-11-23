# An after script runs at level 0 in the global namespace, not in the registering proc's frame.
proc p {} { set x local; after 0 {set ::r1 [list [info level] [namespace current] [info exists x]]} }
namespace eval ns { variable y 1; after 0 {set ::r2 [list [namespace current] [info exists y]]} }
p
after 10 {set ::done 1}
vwait done
puts $r1
puts $r2
