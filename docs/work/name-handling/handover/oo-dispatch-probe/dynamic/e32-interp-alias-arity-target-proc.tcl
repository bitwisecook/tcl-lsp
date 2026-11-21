proc base {a b} { return $a$b }
proc wrap {} { return [base 1] } ;# BAD arity
puts [catch wrap m]:$m
