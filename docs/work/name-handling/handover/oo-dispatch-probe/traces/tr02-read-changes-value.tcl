# A read trace replaces the value before the reader sees it.
proc bump {n1 n2 op} { upvar 1 $n1 v; set v 42 }
set x 1
trace add variable x read bump
puts "read=$x"
puts "expr=[expr {$x + 1}]"
proc p {} { set y 1; trace add variable y read bump; return [expr {$y * 2}] }
puts "proc=[p]"
