# W108: Non-ASCII character outside standard ASCII printable/whitespace set
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:44 (degree sign °)
#
# The LSP flags non-ASCII characters as a portability warning. While Tcl
# handles UTF-8 fine, non-ASCII in source can cause encoding issues when
# files are transferred between systems or editors.
#
# Expected: tclsh runs without error — Tcl handles UTF-8 natively.

set temp 25
set label "${temp}°C"
puts "temperature label: $label"

# The degree sign U+00B0 is flagged
puts "100°F = boiling? No."

# Safe alternative: use escape or ASCII approximation
set label2 "${temp} deg C"
puts "ASCII alternative: $label2"

puts "W108 test complete"
