# interp share/transfer of a channel; chan names per interpreter
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set f [open c21.txt w]
interp create k
t transfer {interp transfer {} $f k; list [expr {$f in [file channels]}] [k eval [list expr [list {$f} in \[file channels\]]]]}
t child-puts {k eval [list puts $f hi]; k eval [list close $f]; file size c21.txt}
set g [open c21.txt r]
t share {interp share {} $g k; list [expr {$g in [file channels]}] [k eval [list gets $g]]}
t parent-close {close $g; k eval [list expr [list {$g} in \[file channels\]]]}
interp delete k
