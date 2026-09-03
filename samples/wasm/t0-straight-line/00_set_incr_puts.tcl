# T0: the minimal program. Every statement must compile to native WASM with no
# Tcl framing: two i64 locals, one boxed write at the puts boundary.
set a 1
incr a
puts $a
