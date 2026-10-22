proc p {d} { dict with d { set a 1 }; dict update d k v { set v 2 }; dict for {k v} $d { set x 1 }; return $d }
if {[catch {puts [::tcl::unsupported::disassemble proc p]}]} { puts "no disassemble" }
