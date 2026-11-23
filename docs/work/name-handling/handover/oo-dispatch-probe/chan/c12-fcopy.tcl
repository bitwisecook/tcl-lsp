# fcopy sync with -size, chan copy async with -command
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set f [open c12a.txt w]; puts -nonewline $f [string repeat x 100]; close $f
t sync {set i [open c12a.txt]; set o [open c12b.txt w]; set n [fcopy $i $o -size 30]; close $i; close $o; list $n [file size c12b.txt]}
t async {set i [open c12a.txt]; set o [open c12c.txt w]; chan copy $i $o -command {set ::done}; vwait ::done; close $i; close $o; list $::done [file size c12c.txt]}
t badopt {set i [open c12a.txt]; catch {fcopy $i stdout -nosuch 1} m; close $i; set m}
