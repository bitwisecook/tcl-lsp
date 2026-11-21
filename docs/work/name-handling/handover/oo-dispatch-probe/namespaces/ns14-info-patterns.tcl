namespace eval ::m { proc p1 {} {}; proc p2 {} {}; variable v1 1; variable v2 2 }
puts [list [lsort [info procs ::m::p*]] [lsort [info vars ::m::v*]] [lsort [namespace eval ::m {info procs}]] [lsort [namespace eval ::m {info vars v*}]]]
