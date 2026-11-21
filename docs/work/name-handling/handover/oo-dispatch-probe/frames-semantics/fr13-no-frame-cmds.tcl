proc p {} { set r [info level]; eval {lappend r [info level]}; catch {lappend r [info level]}; if 1 {lappend r [info level]}; foreach _ 1 {lappend r [info level]}; return $r }
puts [p]
