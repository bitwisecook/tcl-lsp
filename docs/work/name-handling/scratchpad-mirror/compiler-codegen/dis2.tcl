puts [::tcl::unsupported::disassemble script {dict update d k v { set v 2 }; dict with d {}; dict with d {set a 1}}]
proc q {d} { dict with d {}; dict update d k v {}; dict update ::d k v {set x 1}; dict update d k a(1) {set x 1}; dict with ::d {set x 1}; dict with d k {set y 2}}
puts [::tcl::unsupported::disassemble proc q]
