oo::class create C { method m {a} { return $a } }
C create o
puts [o m 1]
puts [catch {o m} m]:$m ;# BAD arity
puts [catch {o nosuch} m]:$m ;# BAD unknown method
