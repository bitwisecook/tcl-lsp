proc p {} {}
rename p q
puts [catch {p} m]:$m ;# CALL p
