# Tcl ``join list`` (no separator arg) defaults to a single space, not
# an empty string. Regression guard for the runtime bug where the
# missing optional arg — filled with 0 by the compiler — produced an
# empty separator and silently concatenated the elements.
set r [join {a b c}]
if {$r ne "a b c"} { error "join {a b c} = '$r', want 'a b c'" }
# Explicit sep still works.
set r2 [join {x y z} -]
if {$r2 ne "x-y-z"} { error "join with '-' = '$r2', want 'x-y-z'" }
set r
