# expr built-in math functions: variadic min/max, int/entier/wide cast,
# bool, pow, abs.  Previously only abs + 2-arg min/max were inlined;
# everything else fell through to ``0``.  Float-valued builtins (sqrt,
# log, sin, ...) still return 0 — they need full float-through-IR
# plumbing and are a separate follow-up.
if {[expr {min(3,1,2)}] != 1} { error "min(3,1,2) != 1" }
if {[expr {max(3,1,2)}] != 3} { error "max(3,1,2) != 3" }
if {[expr {min(1,2,3,4,5)}] != 1} { error "min variadic != 1" }
if {[expr {max(5,4,3,2,1)}] != 5} { error "max variadic != 5" }
if {[expr {abs(-5)}] != 5} { error "abs(-5) != 5" }
if {[expr {abs(5)}] != 5} { error "abs(5) != 5" }
if {[expr {int(42)}] != 42} { error "int(42) != 42" }
if {[expr {entier(42)}] != 42} { error "entier(42) != 42" }
if {[expr {wide(42)}] != 42} { error "wide(42) != 42" }
if {[expr {pow(2,10)}] != 1024} { error "pow(2,10) != 1024" }
if {[expr {bool(5)}] != 1} { error "bool(5) != 1" }
if {[expr {bool(0)}] != 0} { error "bool(0) != 0" }
return ok
