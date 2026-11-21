proc f {a} { return $a }
puts [f 1]
proc f {a b} { return $a$b }
puts [catch {f 1} m]:$m ;# BAD arity changed
puts [f 1 2]
