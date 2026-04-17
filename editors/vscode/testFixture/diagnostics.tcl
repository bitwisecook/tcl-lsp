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
