package require Tk
text .t -yscrollcommand {.s set}
scrollbar .s -command {.t yview}
pack .s -side right -fill y
pack .t -side left -fill both -expand 1
.t insert end [string repeat "line\n" 50]
update
.s set 0 0.5
puts ok
