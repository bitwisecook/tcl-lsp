proc add {a b} { return [expr {$a + $b}] }
set x 1
if {$x > 0} { set y [add $x 2] } else { set y 0 }
puts $y
