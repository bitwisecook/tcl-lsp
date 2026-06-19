proc add {a b} { return [expr {$a + $b}] }
set x 5
if {$x > 0} { set y [add $x 9] } else { set y 1 }
puts $y
switch -glob -- $y { a* { puts A } default { puts $y } }
