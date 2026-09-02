# T3: procs. A leaf proc with two int params should become a native i64
# function; the caller boxes once at the puts boundary.
proc add {a b} { return [expr {$a + $b}] }
proc sq {x} { expr {$x * $x} }
puts [add 2 4]
puts [sq [add 1 2]]
