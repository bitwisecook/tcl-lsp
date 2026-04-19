# W210: Variable read before it is set
# Source: SpiceGenTcl/examples/ngspice/advanced/monte_carlo.tcl:35
#         SpiceGenTcl/pkgIndex.tcl:0 ($dir)
#         SpiceGenTcl/docs/doc_gen.tcl:22,84
#
# The LSP flags variables that are read before any visible assignment.
# Some of these are false positives — e.g. $dir in pkgIndex.tcl is set
# by the package system, and lappend creates a variable if it doesn't exist.
#
# Expected: tclsh runs — lappend auto-creates, but plain $var throws error.

# Case 1: lappend auto-creates (false positive for the LSP)
# This is the pattern from monte_carlo.tcl — lappend creates the var
lappend items "first"
lappend items "second"
puts "lappend auto-create: $items"

# Case 2: $dir in pkgIndex.tcl — set by package system (false positive)
# Simulating: package ifneeded Foo 1.0 [list source [file join $dir foo.tcl]]
# $dir is set by Tcl's package mechanism before evaluating pkgIndex.tcl

# Case 3: genuine read-before-set (true positive)
proc bad_example {} {
    # startPage is read but never set in the visible scope
    if {[catch {set val $startPage}]} {
        puts "genuine read-before-set: error caught (variable doesn't exist)"
    }
}
bad_example

puts "W210 test complete"
