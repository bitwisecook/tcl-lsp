package ifneeded demo 1.0 {package provide demo 1.0; set ::loaded 1.0}
package ifneeded demo 1.5 {package provide demo 1.5; set ::loaded 1.5}
puts [list [package require demo] $::loaded [package present demo] [package require demo 1.0] [catch {package require demo 2.0} m] $m]
