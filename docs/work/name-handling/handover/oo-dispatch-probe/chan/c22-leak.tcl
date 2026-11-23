# channel opened in a proc and never closed stays open (leak)
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
proc leak {} { set f [open c22.txt w]; return }
t before {llength [file channels]}
t leak {leak; llength [file channels]}
