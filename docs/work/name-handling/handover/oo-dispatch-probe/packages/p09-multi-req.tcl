package ifneeded demo 1.5 {package provide demo 1.5}
package ifneeded demo 3.0 {package provide demo 3.0}
puts [package require demo 1.0-2.0 3.0-]
