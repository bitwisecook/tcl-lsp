proc p {} {}
proc tr {old new op} { lappend ::log [list [namespace tail $old] [namespace tail $new] $op] }
trace add command p {rename delete} tr
rename p q
rename q {}
puts $::log
