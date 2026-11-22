proc t {a b} {return $a$b}
interp alias {} al {} t 1
puts [catch {al} m]:$m ;# CALL al
