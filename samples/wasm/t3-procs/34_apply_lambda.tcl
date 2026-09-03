# T3: apply with a literal lambda - compilable as an anonymous proc.
set double {x {expr {$x * 2}}}
puts [apply $double 21]
puts [lmap n {1 2 3} {apply {{x} {expr {$x + 100}}} $n}]
set adder [list {a b} {expr {$a + $b}}]
puts [apply $adder 3 4]
