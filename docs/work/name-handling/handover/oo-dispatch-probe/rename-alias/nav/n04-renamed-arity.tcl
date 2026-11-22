proc p {x} {return $x}
rename p q
puts [catch {q} m]:$m ;# CALL q
