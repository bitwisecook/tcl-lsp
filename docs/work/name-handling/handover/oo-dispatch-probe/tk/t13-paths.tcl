package require Tk
button .b1
puts [catch {button .b1} m]:$m ;# BAD path exists
puts [catch {button .nope.b} m]:$m ;# BAD parent missing
puts [catch {button .Upper} m]:$m ;# BAD upper-case name
destroy .b1
puts [catch {.b1 configure -text x} m]:$m ;# BAD destroyed
