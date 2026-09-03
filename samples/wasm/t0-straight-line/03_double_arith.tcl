# T0: floats. Tests that the native lowering picks f64 and that the string
# rendering at the boundary matches Tcl's shortest-roundtrip formatting.
set r 2.5
set area [expr {3.14159 * $r * $r}]
puts $area
puts [expr {$area > 19.0}]
puts [expr {int($area)}]
