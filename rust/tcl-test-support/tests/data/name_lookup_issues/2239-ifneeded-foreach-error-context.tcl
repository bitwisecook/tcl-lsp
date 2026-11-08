package ifneeded t 2.1 {package provide t 2.1; foreach x 1 {error "ifneeded test" EI}}
puts [list [catch {package require t 2.1} msg] $msg $::errorInfo]
