foreach {v req} {1.5 1.0 2.0 1.0 1.5 1.0-2.0 2.0 1.0-2.0 2.0 1.0-2 1.5 2-1 1.2a1 1.2 1.2b1 1.2a1- 9.0 8.6- 8.6.18 8.6} { lappend r [package vsatisfies $v $req] }
puts $r
