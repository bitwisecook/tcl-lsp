# zlib push gzip on a file channel
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t write {set f [open c15.gz wb]; zlib push gzip $f; puts -nonewline $f hello; close $f; file size c15.gz}
t read {set f [open c15.gz rb]; zlib push gunzip $f; set r [read $f]; close $f; set r}
t pop {set f [open c15.gz rb]; zlib push gunzip $f; chan pop $f; set r [string length [read $f]]; close $f; set r}
