# W101: eval with substituted arguments risks code injection
# Source: SpiceGenTcl/src/generalClasses.tcl:2373,2758
#
# Using eval with variable substitution allows arbitrary code execution
# if the variable contains Tcl commands. The safe alternative is {*}
# expansion or a braced body.
#
# Expected: tclsh runs — demonstrates both the risky and safe patterns.

proc greet {name} {
    return "Hello, $name"
}

# Risky pattern (W101): eval with substituted arguments
set cmd "greet"
set arg "World"
set result [eval $cmd $arg]
puts "eval (risky): $result"

# Safe alternative: use {*} expansion
set cmdList [list greet World]
set result [{*}$cmdList]
puts "{*} (safe):   $result"

# Demonstrate the injection risk
set malicious {[puts "INJECTED!"]}
# This would execute the injected command:
catch {eval greet $malicious} err
puts "injection attempt result: $err"

puts "W101 test complete"
