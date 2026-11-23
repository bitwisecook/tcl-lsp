# tcl_precision: lazily readable in 8.x, changes float formatting; absent in 9.x.
puts "exists=[info exists tcl_precision]"
puts [list read [catch {set tcl_precision} m] $m]
puts "default=[expr {1/3.0}]"
puts [list set [catch {set tcl_precision 3} m] $m]
puts "after=[expr {1/3.0}]"
set tcl_precision 0
catch {unset tcl_precision}
puts [list reread [catch {set tcl_precision} m] $m]
