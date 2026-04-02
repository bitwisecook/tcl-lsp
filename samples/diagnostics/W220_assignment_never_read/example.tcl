# W220: Assignment to variable is never read
# Source: SpiceGenTcl/examples/ltspice/transient/fourbitadder.tcl (tonStep, perStep)
#         SpiceGenTcl/docs/doc_gen.tcl:7 (sourceDir)
#
# The specific assignment is dead — the value written is never consumed.
# Different from W211 in that the variable might be used elsewhere, but
# this particular assignment is wasted.
#
# Expected: tclsh runs without error.

set tonStep "0.5e-6"
set perStep "1e-6"

# tonStep and perStep are set but never read before the proc ends
# In the original code, they were likely meant for pulse definitions

puts "W220 test complete (no output from dead assignments)"
