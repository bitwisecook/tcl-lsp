# A tcl_precision write changes later float formatting (8.x) and is never a dead store.
set tcl_precision 3
puts [expr {1/3.0}]
puts $tcl_precision
