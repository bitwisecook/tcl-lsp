# channel naming for open, second open, after close, file tempfile
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t a {set a [open c03.txt w]}
t b {set b [open c03.txt r]}
t close-a {close $a}
t c {set c [open c03.txt r]}
t tempfile {set tf [file tempfile]; close $tf; string match file* $tf}
t names-file {llength [lsearch -all -glob [file channels] file*]}
