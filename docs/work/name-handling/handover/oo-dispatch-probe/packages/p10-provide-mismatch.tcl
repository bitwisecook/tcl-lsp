package ifneeded demo 1.0 {package provide demo 1.1}
puts [catch {package require demo 1.0} m]:$m
