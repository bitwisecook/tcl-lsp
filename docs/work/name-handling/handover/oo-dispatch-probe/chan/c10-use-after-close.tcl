# use of a channel name after close, and an unknown channel
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t puts {set f [open c10.txt w]; close $f; puts $f x}
t gets {gets file999}
t eof {eof nosuch}
t fconfigure {fconfigure nosuch}
t close-twice {set f [open c10.txt w]; close $f; close $f}
