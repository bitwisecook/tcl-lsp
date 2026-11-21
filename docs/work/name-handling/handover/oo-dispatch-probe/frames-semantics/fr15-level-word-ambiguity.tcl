proc p {} { set 2 two; uplevel 2 }
proc q {} { set r [catch {p} m]; return $r:$m }
puts [q]
proc s {} { uplevel #0 set ::hh 1; return $::hh }
puts [s]
