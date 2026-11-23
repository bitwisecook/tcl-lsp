# Code handling by while, for, foreach, lmap, dict for on custom codes and return.
proc w {} { set i 0; while {$i < 3} { incr i; if {$i == 2} { return -code 5 five } } ; return done }
puts "while custom: [catch {w} r] $r"
proc fr {} { for {set i 0} {$i < 3} {incr i} { if {$i == 1} { return early } } ; return late }
puts "for return: [fr]"
puts "foreach custom at top: [catch {foreach x {1 2} { return -code 6 six }} r] $r"
puts "lmap continue: [lmap x {1 2 3} { if {$x == 2} continue ; set x }]"
puts "lmap break: [lmap x {1 2 3} { if {$x == 2} break ; set x }]"
puts "for next-clause break: [catch {for {set i 0} {$i < 3} {break} { set z $i }} r] $r [info exists z] $z"
