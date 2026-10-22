proc ::tcl::mathfunc::custom {x} {incr ::hit; return $x}
set ::hit 0
set x [expr {custom(3)}]
puts [expr {custom(3)}]
set y "a[expr {custom(3)}]b"
set z [list [expr {custom(3)}]]
puts "hit=$::hit"
