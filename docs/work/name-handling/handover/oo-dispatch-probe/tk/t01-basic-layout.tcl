package require Tk
button .b -text Hi -command {puts clicked}
label .l -text Name
entry .e
grid .l .e -sticky ew
grid .b -columnspan 2
grid columnconfigure . 1 -weight 1
.b invoke
puts [catch {grid .b -sticky nsewx} m]:$m ;# BAD sticky
puts [catch {.b configure -txt x} m]:$m ;# BAD option typo
puts [catch {pack .l -side middle} m]:$m ;# BAD side value
