# Write-trace callback arguments for a scalar, an array element and a whole array.
proc cb {n1 n2 op} { puts "cb {$n1} {$n2} $op" }
set x 0
trace add variable x write cb
set x 1
array set a {k 0}
trace add variable a write cb
set a(k) 2
trace add variable a(j) write cb
set a(j) 3
