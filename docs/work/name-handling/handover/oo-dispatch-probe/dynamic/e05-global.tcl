set g 1
proc p {} { global g; return $g }
proc q {} { return $::g }
puts [p][q]
proc r {} { global nope; return [catch {set nope} m]:$m } ;# BAD read of unset global
puts [r]
