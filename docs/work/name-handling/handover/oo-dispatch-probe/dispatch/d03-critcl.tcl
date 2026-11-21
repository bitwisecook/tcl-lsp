package require critcl
critcl::cproc add {int a int b} int { return a + b; }
puts [add 1 2]
puts [catch {add 1} m]:$m ;# BAD arity
