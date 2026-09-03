# T1: values a conditionally evaluated operand touches. A short-circuited
# right operand and each ternary arm run on one path only, so nothing they
# read may be reused afterwards; a double division that yields NaN is a
# domain error, not a value.
set x 42
puts start
puts [expr {0 && $x}]
puts $x
set y 7
puts guard
puts [expr {1 || $y}]
puts $y
set p 3
set q 9
puts [expr {$x == 42 ? $p : $q}]
puts $p
puts $q
puts [catch {expr {0.0 / 0.0}} m]
puts $m
puts [expr {1.0 / 0.0}]
