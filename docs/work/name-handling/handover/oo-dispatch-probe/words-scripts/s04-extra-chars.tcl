puts [list [catch {set a "x"y} m] $m [catch {set b {x}y} m2] $m2]
