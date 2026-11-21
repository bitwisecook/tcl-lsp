package require Tk
proc cb {x} { puts got:$x }
set name "a b"
button .ok -command [list cb $name]
.ok invoke
button .bad -command "cb $name"
puts [catch {.bad invoke} m]:$m ;# BAD quoted command splits the value
