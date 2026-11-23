# chan pipe (8.6+), open |cmd, pid
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t pipe {lassign [chan pipe] r w; puts $w x; close $w; set l [gets $r]; close $r; set l}
t names {lassign [chan pipe] r w; set n [list [string match file* $r] [string match file* $w]]; close $r; close $w; set n}
t open-pipe {set p [open "|echo hi"]; set l [gets $p]; set pp [llength [pid $p]]; close $p; list $l $pp}
t pid {string is integer [pid]}
