# Jim lambda (a named, GC-collected procedure) with statics
set f [lambda {x} {expr {$x * 2}}]
puts [$f 4]
set counter [lambda {} {{n 0}} {incr n}]
$counter
puts [$counter]
