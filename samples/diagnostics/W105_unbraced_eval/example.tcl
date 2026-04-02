# W105: Code block argument to eval is not braced — double substitution risk
# Source: SpiceGenTcl/src/generalClasses.tcl:2373,2758
#
# When eval's argument contains $var or [cmd] without braces, Tcl performs
# substitution twice: once when building the argument, once inside eval.
# Bracing the body prevents the first substitution.
#
# Expected: tclsh runs without error.

proc greet {name} { return "Hello, $name" }

set funcName "greet"
set arg "World"

# Risky pattern (W105): eval with unbraced substitutions
set result [eval "$funcName $arg"]
puts "unbraced eval: $result"

# Safe pattern: use {*} expansion with a proper list
set result2 [{*}[list $funcName $arg]]
puts "safe {*}: $result2"

# Safe pattern: braced body with internal references
eval {set x 42}
puts "braced eval: x=$x"

puts "W105 test complete"
