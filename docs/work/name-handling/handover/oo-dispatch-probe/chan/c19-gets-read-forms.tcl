# gets/read forms: variable count, -nonewline, read count
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set f [open c19.txt w]; puts $f "ab"; puts $f "cd"; close $f
t gets-var {set f [open c19.txt]; set n [gets $f line]; close $f; list $n $line}
t read-nonl {set f [open c19.txt]; set r [read -nonewline $f]; close $f; set r}
t read-n {set f [open c19.txt]; set r [read $f 4]; close $f; set r}
t eof-gets {set f [open c19.txt]; read $f; set n [gets $f l]; close $f; list $n $l}
t legacy-read-nonl {set f [open c19.txt]; set r [catch {read $f nonewline} m]; close $f; list $r $m}
t puts-legacy {set f [open c19.txt w]; set r [catch {puts $f x nonewline} m]; close $f; list $r $m [file size c19.txt]}
