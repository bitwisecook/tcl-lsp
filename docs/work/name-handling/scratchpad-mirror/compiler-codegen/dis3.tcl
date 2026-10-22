proc c {a b} { set r [string cat a - b]; set s [string cat $a x $b]; set t [string cat]; set u [string cat $a]; return }
proc i {s} { if {![string is ascii $s]} {return 1}; string is integer -strict $s; string is int $s; string is ascii -failindex fi $s; string is boolean $s; string is $s x; string is alpha -strict -strict $s }
foreach p {c i} { if {[catch {puts [::tcl::unsupported::disassemble proc $p]} m]} {puts "ERR $m"} }
