proc bump {name} {upvar 1 $name v; incr v}
proc grow {name} {upvar 1 $name v; set v 1; lappend v 2}
proc drop {name} {upvar 1 $name v; set v 1; unset v}
set n 1; bump n; grow g; drop d
puts "$n [info exists g] $g [info exists d]"
