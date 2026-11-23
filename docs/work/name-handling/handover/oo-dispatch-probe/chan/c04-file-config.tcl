# full fconfigure of a regular file opened for writing and reading
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t w {set f [open c04.txt w]; set r [fconfigure $f]; close $f; set r}
t r {set f [open c04.txt r]; set r [fconfigure $f]; close $f; set r}
t rw {set f [open c04.txt r+]; set r [list [fconfigure $f -translation] [fconfigure $f -eofchar]]; close $f; set r}
t bad-opt {set f [open c04.txt r]; catch {fconfigure $f -nosuch} m; close $f; set m}
