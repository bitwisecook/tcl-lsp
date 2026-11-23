# Default unknown: message, errorCode and errorInfo for an unresolved command.
catch {no_such_command_q 1 2} m
puts "msg: $m"
puts "code: $::errorCode"
puts "info0: [lindex [split $::errorInfo \n] 0]"
puts "unknown is: [info procs unknown] [expr {[info commands unknown] ne {}}]"
