# W001: Unknown subcommand — variable interpolation in subcommand position
# Source: SpiceGenTcl/examples/ngspice/transient/switch_oscillator_rbc.tcl
#
# The LSP warns because it cannot resolve ${widget} to a known subcommand
# of 'grid'. At runtime this is perfectly valid — the variable holds a
# widget path and 'grid' accepts widget paths as arguments.
#
# Expected: tclsh runs without error (Tk required for actual grid layout).

if {[catch {package require Tk} tkError]} {
    puts "W001 test skipped: Tk unavailable ($tkError)"
    exit 0
}

set graph1 [canvas .c1 -width 200 -height 100]
set graph2 [canvas .c2 -width 200 -height 100]
grid ${graph1} -row 0 -column 0
grid ${graph2} -row 1 -column 0

puts "W001 test: grid with variable widget paths — valid Tk usage"
destroy .
