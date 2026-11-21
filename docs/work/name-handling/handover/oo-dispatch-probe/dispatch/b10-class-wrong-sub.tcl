oo::class create C
puts [catch {C nosuch} m]:$m ;# BAD
puts [catch {C new} m]
