package ifneeded Demo 1.0 {package provide Demo 1.0}
puts [list [catch {package require demo} m] [package require Demo]]
