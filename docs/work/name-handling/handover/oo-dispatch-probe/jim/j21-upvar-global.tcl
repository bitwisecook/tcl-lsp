set g 1
proc p {} {global g; incr g; return $g}
puts [p]
proc q {} {upvar 1 g h; set h 9}
q
puts $g
puts [catch {set ::g} r]:$r
