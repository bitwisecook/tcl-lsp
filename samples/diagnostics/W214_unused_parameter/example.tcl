# W214: Parameter of proc is unused
# Source: SpiceGenTcl/examples/ngspice/advanced/monte_carlo.tcl (re, im unused)
#         SpiceGenTcl/examples/ngspice/advanced/inverter_optimization.tcl (width, length)
#
# The proc declares parameters that are never referenced in the body.
# This may indicate an incomplete implementation or a wrong signature.
#
# Expected: tclsh runs without error.

proc calcDbMag {re im} {
    # Both 're' and 'im' are unused — the proc uses hardcoded values
    set mag 1.5
    set db [expr {10 * log10($mag)}]
    return $db
}

proc pdPsCalc {width length} {
    # Both parameters unused
    return 42
}

puts "calcDbMag: [calcDbMag 3.0 4.0]"
puts "pdPsCalc:  [pdPsCalc 10 20]"

puts "W214 test complete"
