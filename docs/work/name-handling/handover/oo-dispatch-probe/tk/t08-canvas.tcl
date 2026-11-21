package require Tk
canvas .c -width 200 -height 100
set id [.c create rectangle 10 10 50 50 -fill red -tags box]
.c itemconfigure $id -outline blue
.c bind box <1> {puts hit}
.c move box 5 5
puts [.c coords $id]
puts [catch {.c create rectangle 10 10 -fill red} m]:$m ;# BAD coords
puts [catch {.c itemconfigure $id -colour red} m]:$m ;# BAD item option
puts [catch {.c create blob 1 1} m]:$m ;# BAD item type
