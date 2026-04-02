# W112: Trailing whitespace
# Source: many files in SpiceGenTcl (SpiceGenTcl.tcl, examples, src)
#
# Trailing whitespace is a style issue flagged as HINT. No runtime impact.
# The trailing spaces on lines below are intentional to trigger the diagnostic.
#
# Expected: tclsh runs without error.

set x 1   
set y 2     
puts "x=$x y=$y"

puts "W112 test complete"
