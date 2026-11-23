# 9.0: tcl_precision no longer controls double formatting
set tcl_precision 3
puts [expr {1.0/3}]
set tcl_precision 0
puts [expr {1.0/3}]
