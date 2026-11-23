# interp share/transfer of a channel between interpreters (checked with eof, not file channels)
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
proc has {i c} { expr {![catch {interp eval $i [list eof $c]}]} }
set f [open c21.txt w]
interp create k
t transfer {interp transfer {} $f k; list [has {} $f] [has k $f]}
t child-puts {k eval [list puts $f hi]; k eval [list close $f]; file size c21.txt}
set g [open c21.txt r]
t share {interp share {} $g k; list [has {} $g] [k eval [list gets $g]]}
t parent-close {close $g; has k $g}
interp delete k
