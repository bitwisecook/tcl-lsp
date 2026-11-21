proc old {a} { return $a }
rename old new
puts [new 1]
puts [catch {old 1} m]:$m ;# BAD renamed away
