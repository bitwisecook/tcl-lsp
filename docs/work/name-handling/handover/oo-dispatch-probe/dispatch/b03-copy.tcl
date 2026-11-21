oo::class create C { method m {} { return m } }
set o [C new]
oo::copy $o o2
puts [o2 m]
puts [catch {o2 nosuch} m]:$m ;# BAD
