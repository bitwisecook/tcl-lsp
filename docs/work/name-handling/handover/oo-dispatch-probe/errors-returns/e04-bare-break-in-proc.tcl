# A bare break or continue leaving a procedure body is an error, not a loop exit in the caller.
proc b {} { break }
proc c {} { continue }
set out {}
foreach i {0 1} { set code [catch {b} msg]; lappend out $code $msg }
puts "b: $out"
puts "c: [list [catch {c} msg] $msg]"
puts "info: [lindex [split $::errorInfo \n] 0]"
puts "toplevel-catch: [catch {break}]"
