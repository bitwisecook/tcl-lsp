set l {a b c d e}
foreach i {0 end end-1 end+1 -1 1+1 end-1-1 01 0x1 " 1" 5 end--1} { lappend r [list $i [catch {lindex $l $i} v] $v] }
puts $r
