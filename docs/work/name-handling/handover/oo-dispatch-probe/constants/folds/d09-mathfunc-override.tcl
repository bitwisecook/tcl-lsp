proc ::tcl::mathfunc::double {x} { return 42 }
proc p {} { return [expr {double(1)}] }
puts [p]
