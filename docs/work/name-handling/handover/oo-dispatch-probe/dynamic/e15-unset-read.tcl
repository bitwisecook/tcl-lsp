proc p {} { set v 1; unset v; return [catch {set v} m]:$m } ;# BAD read after unset
puts [p]
