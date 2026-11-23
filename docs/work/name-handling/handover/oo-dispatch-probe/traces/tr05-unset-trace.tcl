# Unset traces fire on unset, on proc-local teardown, and carry the array element name.
proc cb {n1 n2 op} { puts "cb {$n1} {$n2} $op" }
set x 1
trace add variable x unset cb
unset x
proc p {} { set loc 1; trace add variable loc unset cb; puts "in p" }
p
puts "after p"
array set a {k 1 j 2}
trace add variable a unset cb
unset a(k)
unset a
