# Fixture for the diagnostics deep review 2: tight ranges for the
# expression-shimmer operand (S100), the destructive-file path argument
# (W313), the string-compare operator (W110), and I230 message fidelity.
# Line numbers are load-bearing — precisionReview2.test.ts asserts on them.
set s "hello"
set y [expr {$s + 1}]
puts $y
set p [gets stdin]
file delete -force $p
set n 1
if $n { puts a } else { puts b }
set c x
if {$c == "x"} { puts hi }
