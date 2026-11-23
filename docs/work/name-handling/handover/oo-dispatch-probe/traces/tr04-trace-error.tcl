# An error in a variable trace is reported as can't set/read; the write itself has happened.
proc boom {n1 n2 op} { error "nope-$op" }
set x 1
trace add variable x write boom
puts [list [catch {set x 2} m] $m]
trace remove variable x write boom
puts "x=$x"
trace add variable x read boom
puts [list [catch {set y $x} m] $m]
puts [list [info exists y]]
