package ifneeded demo 1.0 {package provide demo 1.0}
package require demo
package forget demo
puts [list [catch {package present demo} m] [package versions demo]]
