proc fix {n1 n2 op} { upvar 1 $n1 v; set v fixed }
proc p {} { trace add variable t write fix; set t raw; return $t }
puts [p]
