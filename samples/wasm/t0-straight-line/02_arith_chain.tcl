# T0: straight-line integer arithmetic. All values provably fit i64; the whole
# chain should be native i64 ops with a single box at the boundary.
set x 10
set y 3
set z [expr {$x * $y + 7}]
set z [expr {$z - $x / $y}]
incr z -1
puts $z
puts [expr {$z % 5}]
