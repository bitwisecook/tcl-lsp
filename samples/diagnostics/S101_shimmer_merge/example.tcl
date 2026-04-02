# S101: Shimmer — variable changes internal representation at merge point
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:30
#
# In Tcl, every value has an internal representation (intrep). When a
# value is used as a string and then as a list (or vice versa), Tcl
# must re-parse it — this is called "shimmering". In hot loops, this
# wastes CPU time re-converting between representations.
#
# Expected: tclsh runs without error — demonstrates the shimmer pattern.

# Build a list by appending formatted strings (string intrep)
# Then use it as a list (list intrep) — causes shimmer
set xydata {}
foreach x {1.0 2.0 3.0} y {0.5 1.5 2.5} {
    set xf [format "%.3f" $x]
    set yf [format "%.3f" $y]
    # lappend treats xydata as a list
    lappend xydata [list $xf $yf]
}

# Now use xydata as a string (triggers shimmer back to string)
puts "as string: $xydata"

# And as a list again (shimmer back to list)
puts "first element: [lindex $xydata 0]"
puts "length: [llength $xydata]"

# The fix: keep the variable in one representation consistently
# Either always use list operations, or convert once at the end.

puts "S101 test complete"
