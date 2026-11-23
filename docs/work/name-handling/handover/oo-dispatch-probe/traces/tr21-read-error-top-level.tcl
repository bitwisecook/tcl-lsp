# A read-trace error on a global, caught at top level and inside a procedure.
proc boom {n1 n2 op} { error "nope-$op" }
set x 1
trace add variable x read boom
proc p {} { list [catch {set q $::x} m] $m }
puts [list proc [p]]
puts [list top [catch {set y $x} m] $m]
