# error msg info code: errorInfo seeding and errorCode.
proc inner {} { error "bad thing" "seeded info" {APP BAD} }
proc outer {} { inner }
catch {outer} m
puts "msg: $m"
puts "code: $::errorCode"
puts "info: [string map [list \n |] $::errorInfo]"
catch {error plain} m
puts "plain code: $::errorCode"
catch {set nosuch} m
puts "readvar code: $::errorCode"
catch {expr {1/0}} m
puts "divzero code: $::errorCode"
