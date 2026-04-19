# Tcl ``foreach {var1 var2 ...} list body`` groups the list in n-tuples
# where n is the number of variables.  Regression for the codegen bug
# where the compiled loop bound only the first variable and advanced by
# 1 element per iteration, leaving later variables empty on every
# iteration.
set pairs {}
foreach {a b} {1 2 3 4 5 6} {lappend pairs $a-$b}
if {$pairs ne {1-2 3-4 5-6}} { error "multi-var foreach pairs wrong: $pairs" }

# Trailing element receives "": list length not a multiple of var count.
set triples {}
foreach {a b c} {1 2 3 4 5 6 7} {lappend triples $a-$b-$c}
if {$triples ne {1-2-3 4-5-6 7--}} { error "triples wrong: $triples" }

# Single-var still works (regression guard for the fix path).
set single {}
foreach x {A B C} {lappend single $x}
if {$single ne {A B C}} { error "single-var foreach wrong: $single" }
return ok
