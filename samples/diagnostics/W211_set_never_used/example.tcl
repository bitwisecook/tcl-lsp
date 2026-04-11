# W211: Variable is set but never used
# Source: SpiceGenTcl/examples/ngspice/advanced/inverter_optimization.tcl
#         SpiceGenTcl/examples/ltspice/transient/fourbitadder.tcl
#
# A variable is assigned but never referenced afterward. This may indicate
# dead code, a typo, or a variable kept for debugging.
#
# Expected: tclsh runs without error — just wastes memory.

set fileDataPath "/some/path/that/is/never/used"
set inpFreq 1000
set noPeriods 10
set tmeasStart 0.0
set tmeasStop 1.0

# None of the above are ever read — they are dead assignments
set actuallyUsed 42
puts "used: $actuallyUsed"

puts "W211 test complete"
