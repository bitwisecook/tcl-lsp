proc W {n1 n2 op} { puts "W $n1 $n2 $op" }
proc E {n1 n2 op} { puts "E $n1 $n2 $op" }
array set a {}
trace add variable a write W
trace add variable a(k) write E
set a(k) 1
puts "= reg order reversed"
array set b {}
trace add variable b(k) write E
trace add variable b write W
set b(k) 2
puts "= multiple write on same var"
proc t1 args {puts 1}
proc t2 args {puts 2}
proc t3 args {puts 3}
trace add variable v write t1
trace add variable v write t2
trace add variable v write t3
set v x
puts "= read"
trace add variable r read t1
trace add variable r read t2
trace add variable r read t3
set r 0
puts [set r]
puts "= unset"
trace add variable u unset t1
trace add variable u unset t2
trace add variable u unset t3
set u 0
unset u
puts "= array trace"
proc a1 args {puts A1}
proc a2 args {puts A2}
trace add variable arr array a1
trace add variable arr array a2
array names arr
puts "= cmd rename/delete"
proc victim {} {}
proc c1 args {puts "c1 $args"}
proc c2 args {puts "c2 $args"}
trace add command victim {rename delete} c1
trace add command victim {rename delete} c2
rename victim victim2
rename victim2 {}
puts "= exec enter/leave"
proc target {} {return T}
proc e1 args {puts "e1 [lindex $args end]"}
proc e2 args {puts "e2 [lindex $args end]"}
trace add execution target {enter leave} e1
trace add execution target {enter leave} e2
target
puts "= trace info variable order"
puts [trace info variable v]
puts [trace info command victim2]
