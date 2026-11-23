# seek/tell/eof/truncate/pending on a file
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set f [open c11.txt w+]
t puts {puts -nonewline $f abcdefghij}
t tell {tell $f}
t seek {seek $f 2 start; tell $f}
t read3 {read $f 3}
t seek-end {seek $f -1 end; read $f}
t eof {eof $f}
t truncate {chan truncate $f 4; seek $f 0; read $f}
t pending {chan pending input $f}
t badseek {seek $f 0 sideways}
t blocked {fblocked $f}
close $f
