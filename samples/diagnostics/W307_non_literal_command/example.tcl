# W307: Non-literal command name — cannot statically analyze
# Source: SpiceGenTcl/examples/ngspice/advanced/monte_carlo.tcl:107,143
#         SpiceGenTcl/src/generalClasses.tcl:785,1777 etc.
#         SpiceGenTcl/examples/ngspice/transient/switch_oscillator_rbc.tcl:78-85
#
# When a command name is stored in a variable (e.g. $cmd arg1 arg2), the
# LSP cannot determine which command will be called. This is valid Tcl
# but prevents static analysis.
#
# Expected: tclsh runs without error.

# Pattern 1: command name in a variable
set cmd "puts"
$cmd "called via variable: works fine"

# Pattern 2: computed method/command dispatch
proc handler_a {x} { return "A: $x" }
proc handler_b {x} { return "B: $x" }

foreach name {a b} {
    set handler "handler_$name"
    puts [$handler "test"]
}

# Pattern 3: object method dispatch (from SpiceGenTcl)
# Simulating: $obj someMethod $args
proc make_greeter {} {
    return "greeter"
}
proc greeter_hello {name} { return "Hello, $name" }

set obj [make_greeter]
set method "${obj}_hello"
puts [$method "World"]

puts "W307 test complete"
