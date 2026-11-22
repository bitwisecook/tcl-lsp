package ifneeded demo 1.0 {package provide demo 1.0}
package ifneeded demo 1.5 {package provide demo 1.5}
puts [list [package require -exact demo 1.0] [catch {package require demo 1.5} m] $m]
