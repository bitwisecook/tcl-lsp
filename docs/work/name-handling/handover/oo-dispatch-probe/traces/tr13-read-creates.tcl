# A read trace on an unset variable can create it; even info exists fires it (tclsh prints exists-before=1).
proc mk {n1 n2 op} { upvar 1 $n1 v; set v made }
trace add variable y read mk
puts "exists-before=[info exists y]"
puts "y=$y"
proc p {} { trace add variable z read mk; return $z }
puts "p=[p]"
