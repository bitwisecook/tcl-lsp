# Tcl 8.x 'trace variable/vinfo/vdelete' forms.
proc cb {n1 n2 op} { puts "cb {$n1} {$n2} $op" }
set x 1
puts [catch {trace variable x w cb} m]:$m
puts [catch {trace vinfo x} m]:$m
set x 2
puts [catch {trace vdelete x w cb} m]:$m
puts [trace info variable x]
