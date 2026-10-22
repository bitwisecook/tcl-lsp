proc p {} {set r [catch {upvar 1 b} m]; return "$r:$m:[info exists b]"}
set 1 one
set b bee
puts [p]
proc q {} {upvar 1 b; return [info exists b]}
