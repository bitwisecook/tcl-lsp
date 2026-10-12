set ::cell 1
set ::reads 0
set ::writes 0
proc ::observe_cell {name index operation} {
    if {$operation == "r" || $operation == "read"} {incr ::reads}
    if {$operation == "w" || $operation == "write"} {incr ::writes}
}
set supported [catch {trace variable ::cell rw ::observe_cell} message]
if {$supported} {set supported [catch {trace add variable ::cell {read write} ::observe_cell} message]}
if {$supported} {puts [list unsupported $supported $message]; exit}
set result [set ::cell 7]
puts [list write $result $::reads $::writes]
set result [set ::cell]
puts [list read $result $::reads $::writes]
set result [incr ::cell]
puts [list update $result $::reads $::writes]
