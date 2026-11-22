set a 2; set l {1}
puts [list [expr {$a == $a in $l}] [expr {"b" eq "b" in {1}}] [expr {"x" in {x} == 1}] [expr {1 != 1 ni {0}}] [expr {2 < 3 in {1}}]]
