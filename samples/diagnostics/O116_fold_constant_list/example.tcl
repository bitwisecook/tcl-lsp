# O116: Fold constant list command
# Source: SpiceGenTcl/examples/ngspice/dc/diode_iv.tcl:16
#         SpiceGenTcl/examples/ngspice/transient/fourbitadder.tcl:98,130
#
# A [list ...] call with all-literal arguments can be folded at compile
# time into a constant string, avoiding runtime list construction.
#
# Expected: tclsh runs without error — both produce the same result.

# Before (O116): list with all constants — can be folded
set temps [list -55 25 85 125 175]
puts "list command: $temps"

# After: the optimiser would replace with the literal
set temps {-55 25 85 125 175}
puts "literal:      $temps"

# Another example from fourbitadder
set params [list -pname Vhi -pval 5]
puts "list params: $params"

# Folded:
set params {-pname Vhi -pval 5}
puts "literal params: $params"

puts "O116 test complete"
