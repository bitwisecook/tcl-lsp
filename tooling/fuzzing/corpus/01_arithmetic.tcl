# Basic arithmetic and variable assignment
set x 42
set y [expr {$x + 8}]
set z [expr {$x * $y - 10}]
set result [expr {$z / 5}]
set mod [expr {$z % 7}]
