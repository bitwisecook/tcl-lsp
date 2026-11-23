# Unsetting interpreter-provided globals.
puts [list [catch {unset tcl_platform} m] $m [info exists tcl_platform]]
puts [list [catch {unset argv} m] $m [info exists argv]]
puts [list [catch {unset env} m] $m [info exists env]]
set env(TCL_LSP_AFTER) 1
puts "child=[exec sh -c {echo x${TCL_LSP_AFTER}x}]"
