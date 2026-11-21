proc tr {n1 n2 op} { upvar 1 $n1 v; set ::seen $v:[info level] }
proc p {} { set t 5; trace add variable t write tr; set t 6; return $::seen }
puts [p]
