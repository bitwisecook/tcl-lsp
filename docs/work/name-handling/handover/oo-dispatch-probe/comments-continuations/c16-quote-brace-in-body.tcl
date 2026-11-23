# A brace inside quotes inside a braced body is still counted
puts [catch {eval {proc q {} { puts "{" }}} r]:$r
proc q2 {} { puts "\{" }
q2
