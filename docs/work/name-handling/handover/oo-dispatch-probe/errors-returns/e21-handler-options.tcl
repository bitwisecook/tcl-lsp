# Options seen by a try handler and by catch for the same error.
proc boom {} { error boom {} {B C} }
try { boom } on error {m o} { puts "try keys: [lsort [dict keys $o]] code=[dict get $o -errorcode] line=[dict get $o -errorline]" }
catch { boom } m o
puts "catch keys: [lsort [dict keys $o]]"
puts "level-in-proc: [proc lv {} { catch {return -level 1 x} m o; dict get $o -level }; lv]"
