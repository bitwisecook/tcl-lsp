proc make {} { proc made {a} { return $a } }
make
puts [made 1]
puts [catch {made} m]:$m ;# BAD arity
