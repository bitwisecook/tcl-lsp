# enterstep/leavestep see every command executed inside the procedure.
proc st {cmd args} { puts "[lindex $args end] {$cmd}" }
proc g {} { set a 1; incr a; return $a }
trace add execution g {enterstep leavestep} st
puts [g]
