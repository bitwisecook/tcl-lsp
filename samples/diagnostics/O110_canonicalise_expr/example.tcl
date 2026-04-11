# O110: Canonicalise expression (InstCombine)
# Source: SpiceGenTcl/examples/ngspice/transient/switch_oscillator.tcl:38
#
# The optimiser canonicalises comparison expressions like {$i<16} into
# a normalised form. This is an instruction combining pass that
# simplifies or normalises expressions for better bytecode.
#
# Expected: tclsh runs without error.

# Pattern from switch_oscillator.tcl: for loop with expression
set result [list]
for {set i 1} {$i<16} {incr i} {
    lappend result $i
}
puts "loop result length: [llength $result]"

# Another canonicalisable expression: operand reordering
set x 10
if {0 < $x} {
    puts "x is positive"
}

puts "O110 test complete"
