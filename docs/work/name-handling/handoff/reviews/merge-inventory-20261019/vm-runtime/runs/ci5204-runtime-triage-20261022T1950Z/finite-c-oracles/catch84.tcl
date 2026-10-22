puts [list catch_error [list [catch {catch {set x 1} r o} m] $m $::errorCode]]
puts [list catch_success [list [catch {set x 1} r] $r]]
