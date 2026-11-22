package ifneeded demo 1.0 {package provide demo 1.0}
package ifneeded demo 2.0b1 {package provide demo 2.0b1}
puts [list [package prefer] [package require demo]]
package forget demo
package ifneeded demo 1.0 {package provide demo 1.0}
package ifneeded demo 2.0b1 {package provide demo 2.0b1}
package prefer latest
puts [list [package prefer] [package require demo]]
