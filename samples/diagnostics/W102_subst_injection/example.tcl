# W102: subst with a variable argument enables code injection
# Source: SpiceGenTcl/src/generalClasses.tcl:2853,2860,2915
#
# subst evaluates $var and [cmd] inside the string. If the variable
# contains untrusted input, this is a code injection vector.
# The safe alternatives are -nocommands/-novariables, format, or string map.
#
# Expected: tclsh runs — shows the risk and safe alternatives.

set name "World"
set template {Hello, $name}

# Risky pattern (W102): subst with variable
set result [subst $template]
puts "subst (risky): $result"

# Safe alternative: subst with restrictions
set result2 [subst -nocommands $template]
puts "subst -nocommands: $result2"

# Safe alternative: string map
set result3 [string map [list {$name} $name] {Hello, $name}]
puts "string map: $result3"

# Demonstrate the injection risk
set evil {$name [puts "INJECTED"]}
puts "injection attempt:"
catch {subst $evil}

puts "W102 test complete"
