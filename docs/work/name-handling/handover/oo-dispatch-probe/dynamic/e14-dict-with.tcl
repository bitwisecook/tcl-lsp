proc p {} { set d {a 1 b 2}; dict with d {}; return $a$b }
puts [p]
proc q {} { set d {a 1}; dict with d {}; return [catch {set b} m]:$m } ;# BAD b not a key
puts [q]
