# T7: variable traces. Every compiled read/write of a traced cell must go
# through the runtime; untraced cells elsewhere in the script may stay native.
set log {}
proc watch {name1 name2 op} { lappend ::log "$op $name1" }
set a 1
set b 1
trace add variable a write watch
trace add variable a read watch
set a 2
incr a
set c $a
incr b
puts $log
puts "$a $b $c"
trace remove variable a write watch
set a 5
puts $log
