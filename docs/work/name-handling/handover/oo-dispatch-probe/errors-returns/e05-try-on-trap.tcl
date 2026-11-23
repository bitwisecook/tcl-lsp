# try: handler selection, trap prefix matching, finally ordering, custom codes.
proc t {script} {
  set log {}
  set c [catch {try $script on ok {v} { lappend log ok:$v } trap {A B} {m o} { lappend log trapAB:$m } trap {A} {m} { lappend log trapA:$m } on error {m} { lappend log err:$m } on break {} { lappend log brk } on 5 {v} { lappend log five:$v } finally { lappend log fin }} r]
  return "$log => $c [list $r]"
}
puts [t {set x 1}]
puts [t {throw {A B C} abc}]
puts [t {throw {A X} ax}]
puts [t {error plain}]
puts [t {break}]
puts [t {return -level 0 -code 5 v5}]
puts [t {return -level 0 -code 6 v6}]
puts [t {return -code 5 viaLevel1}]
puts [t {continue}]
