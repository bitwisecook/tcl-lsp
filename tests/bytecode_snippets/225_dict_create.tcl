# ``dict create k1 v1 k2 v2 ...`` must return the k/v pairs as a list.
# Previously the runtime helper ignored its arguments and always
# returned the empty string, so ``dict create a 1 b 2`` produced "".
if {[dict create a 1 b 2] ne "a 1 b 2"} { error "literal kv: [dict create a 1 b 2]" }
if {[dict create] ne ""} { error "empty dict create not empty" }
set d [dict create x 10 y 20]
if {[dict get $d y] != 20} { error "round-trip through dict get broke" }
if {[dict size $d] != 2} { error "dict size wrong" }

# With a variable value the compile-time fold should still work via
# a tcl_concat chain.
set k apple
set d2 [dict create $k 7]
if {[dict get $d2 apple] != 7} { error "non-literal key broke" }
return ok
