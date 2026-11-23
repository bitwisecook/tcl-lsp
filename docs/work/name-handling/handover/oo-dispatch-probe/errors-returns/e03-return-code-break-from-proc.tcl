# return -code break/continue from a procedure acts on the caller's loop.
proc stopper {i} { if {$i == 2} { return -code break } ; return $i }
proc skipper {i} { if {$i == 1} { return -code continue } ; return $i }
set out {}
foreach i {0 1 2 3} { lappend out [stopper $i] }
puts "break: $out"
set out {}
foreach i {0 1 2 3} { lappend out [skipper $i] }
puts "continue: $out"
set out {}
foreach i {0 1 2 3} { set v [stopper $i]; lappend out $v }
puts "break2: $out"
