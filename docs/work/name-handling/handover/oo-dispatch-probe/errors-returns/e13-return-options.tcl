# return -options and -level 0; rethrowing with the saved options.
proc rethrow {} { catch {error orig {} {ORIG CODE}} m o; return -options $o $m }
catch {rethrow} m o2
puts "rethrow: $m [dict get $o2 -errorcode] [dict get $o2 -level]"
puts "lvl0: [catch {return -level 0 -code 3} r] [list $r]"
puts "lvl0err: [catch {return -level 0 -code error -errorcode {X Y} e} r] $::errorCode"
proc twolevels {} { inner2; return notreached }
proc inner2 {} { return -level 2 fromInner }
puts "level2: [twolevels]"
proc lvl2break {} { return -level 2 -code break }
proc caller {} { set out {}; foreach i {0 1 2} { lappend out $i; lvl2break } ; return "after:$out" }
set out {}
foreach i {0 1 2} { lappend out [caller] }
puts "level2-break: $out"
