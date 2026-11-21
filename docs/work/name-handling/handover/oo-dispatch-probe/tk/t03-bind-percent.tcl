package require Tk
entry .e
proc onkey {w k} { puts $w:$k }
bind .e <Return> {onkey %W %K}
event generate .e <Return> -when now
bind .e <Escape> {onkey %W} ;# BAD arity, raised as a background error
puts [catch {event generate .e <Escape> -when now} m]:$m ;# BAD arity in binding script
puts [catch {bind .e <Retrun> {onkey %W %K}} m]:$m ;# BAD event name
