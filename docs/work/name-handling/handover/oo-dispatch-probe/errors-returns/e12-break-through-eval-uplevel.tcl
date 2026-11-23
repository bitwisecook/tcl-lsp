# break and continue through eval, uplevel, catch and if inside a loop.
set out {}
foreach i {0 1 2 3} { if {$i == 2} { eval {break} } ; lappend out $i }
puts "eval: $out"
proc ub {} { uplevel 1 {break} }
set out {}
foreach i {0 1 2 3} { if {$i == 1} { set c [catch {ub} m]; lappend out "c$c:$m" } ; lappend out $i }
puts "uplevel-in-proc: $out"
set out {}
foreach i {0 1 2} { lappend out [catch {break}] }
puts "catch-swallows: $out"
set out {}
foreach i {0 1 2 3} { switch $i { 1 { continue } 2 { break } } ; lappend out $i }
puts "switch: $out"
set out {}
foreach i {0 1 2} { uplevel #0 {if {$i == 1} continue}; lappend out $i }
puts "uplevel0-toplevel: $out"
