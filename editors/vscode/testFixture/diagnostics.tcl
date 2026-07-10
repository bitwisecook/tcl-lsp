# File with known diagnostic triggers for testing

# W100 - unbraced expr (double substitution risk)
set x [expr $a + $b]

# W101 - eval with substituted arguments (code injection risk)
eval "puts $x"

# W302 - catch without result variable (silently swallows errors)
catch {error "boom"}

# W110 - string comparison with == in expr
if {$x == "foo"} { puts yes }

# W304 - option-bearing command without -- for dynamic input
regexp $pattern $x

# T102 - tainted data in option position without -- terminator
set tainted_pattern [gets stdin]
regexp $tainted_pattern $x

# E100 - stray close bracket, missing opening '['
set y string]

# E102 - stray close brace, missing opening '{'
set z 1
}
