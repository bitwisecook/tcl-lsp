# A write trace rewrites the value; set returns the post-trace value.
proc clamp {n1 n2 op} { upvar 1 $n1 v; if {$v > 10} { set v 10 } }
set x 0
trace add variable x write clamp
puts "set-result=[set x 99]"
puts "after=$x"
incr x 5
puts "incr=$x"
