# Tcl 9 options: isbinary, -inputmode, -profile, -eofchar single
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t isbinary {set f [open c16.txt wb]; set r [chan isbinary $f]; close $f; set r}
t isbinary-text {set f [open c16.txt w]; set r [chan isbinary $f]; close $f; set r}
t inputmode {fconfigure stdin -inputmode}
t profile {set f [open c16.txt w]; set r [fconfigure $f -profile]; close $f; set r}
t eofchar-pair {set f [open c16.txt w]; set r [catch {fconfigure $f -eofchar {a b}} m]; close $f; list $r $m}
t binary-enc {set f [open c16.txt w]; set r [catch {fconfigure $f -encoding binary} m]; set e [fconfigure $f -encoding]; close $f; list $r $e}
