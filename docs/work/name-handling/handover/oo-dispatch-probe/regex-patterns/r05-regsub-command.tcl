# regsub -command (Tcl 8.7/9.0): subSpec becomes a command prefix called with the match and submatches
proc up {m args} { return [string toupper $m]:[llength $args] }
puts [catch {regsub -all -command {(a)(n)} "banana" up} r]$r
puts [catch {regsub -command {\w+} "hello world" {string length}} r]$r
puts [catch {regsub -all -command {a} "banana" {apply {{m} {format <%s> $m}}}} r]$r
puts [catch {regsub -command {(a)} "banana" up out} r]$r:[expr {[info exists out] ? $out : "-"}]
puts [catch {regsub -all -command {x} "banana" up} r]$r
