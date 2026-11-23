# info errorstack and -errorstack (8.6+).
proc a {x} { b [expr {$x + 1}] }
proc b {y} { error "fail $y" }
catch {a 1} m o
puts "opt: [dict get $o -errorstack]"
puts "info: [info errorstack]"
