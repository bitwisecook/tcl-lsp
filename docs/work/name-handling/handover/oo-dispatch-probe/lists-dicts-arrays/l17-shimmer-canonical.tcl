set s " a  {b}  c "
puts [list [llength $s] [lindex $s 1] [lrange $s 0 end] [string length $s]]
set d " k   v "
puts [list [dict get $d k] [dict size $d] $d [dict set d k w]]
