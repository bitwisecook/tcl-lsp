# O106: Loop-invariant expression re-computed on each iteration
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:29
#         SpiceGenTcl/examples/ngspice/ac/diode_cv.tcl:36
#
# An expression or command call inside a loop produces the same result
# every iteration. Hoisting it before the loop avoids redundant work.
#
# Expected: tclsh runs without error — both versions produce same output.

set pi 3.14159265358979

# Before (O106): format with loop-invariant expression inside loop
set values {1.0 2.0 3.0 4.0 5.0}
foreach y $values {
    # This format call uses $pi and constants — invariant per-iteration
    # only $y changes, but the format spec "%.2e" is the same each time
    set result [format "%.2e" [expr {-$y / (2 * $pi * 1e5 * 1e-9)}]]
    lappend results $result
}
puts "loop-invariant: $results"

# After (hoisted): pre-compute the divisor
unset results
set divisor [expr {2 * $pi * 1e5 * 1e-9}]
foreach y $values {
    set result [format "%.2e" [expr {-$y / $divisor}]]
    lappend results $result
}
puts "hoisted:        $results"

puts "O106 test complete"
