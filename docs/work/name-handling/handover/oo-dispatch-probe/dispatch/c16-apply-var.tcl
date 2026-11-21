set lam {{a} { return $a }}
puts [apply $lam 1]
puts [catch {apply $lam} m]:$m ;# BAD
