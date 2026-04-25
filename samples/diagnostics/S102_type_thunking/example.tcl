# S102: Type thunking — variable oscillates between representations each loop
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:22
#
# The variable alternates between string and list intrep on each loop
# iteration, causing continuous re-parsing. This is worse than S101
# because it happens every iteration, not just at boundaries.
#
# Expected: tclsh runs without error — shows the oscillation pattern.

# Pattern from diode_iv.tcl: build xydata as list, then use as string
set xydata ""
foreach x {1.0 2.0 3.0 4.0 5.0} y {0.1 0.2 0.3 0.4 0.5} {
    set xf [format "%.3f" $x]
    set yf [format "%.3f" [expr {-$y}]]
    # lappend forces list intrep
    lappend xydata [list $xf $yf]
    # if we did: set xydata "$xydata ..." — forces string intrep
    # the oscillation between the two causes type thunking
}
puts "xydata: $xydata"

# Better pattern: stay in list intrep throughout the loop
set xydata_clean [list]
foreach x {1.0 2.0 3.0 4.0 5.0} y {0.1 0.2 0.3 0.4 0.5} {
    set xf [format "%.3f" $x]
    set yf [format "%.3f" [expr {-$y}]]
    lappend xydata_clean [list $xf $yf]
}
puts "clean:  $xydata_clean"

puts "S102 test complete"
