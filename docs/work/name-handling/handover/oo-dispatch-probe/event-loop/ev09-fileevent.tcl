# fileevent and chan event on a pipe: the handler runs in the global frame when readable.
set f [open |[list echo hello] r]
proc onread {ch} { if {[gets $ch line] >= 0} { lappend ::lines $line [info level] } else { close $ch; set ::done 1 } }
if {[llength [info commands chan]]} { chan event $f readable [list onread $f] } else { fileevent $f readable [list onread $f] }
puts [string length [fileevent $f readable]]
after 2000 {set ::done timeout}
vwait done
puts $lines
