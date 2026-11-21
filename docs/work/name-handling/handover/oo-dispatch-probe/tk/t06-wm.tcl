package require Tk
wm title . App
wm geometry . 300x200+10+10
wm protocol . WM_DELETE_WINDOW {puts closing}
wm minsize . 100 100
puts [catch {wm geometry . 300by200} m]:$m ;# BAD geometry
puts [catch {wm protocol . WM_DELETE} m]:$m
