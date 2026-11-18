proc bump {name} {upvar 1 $name v; incr v}
proc reset {name} {upvar 1 $name v; unset v}
proc renew {name} {upvar 1 $name x; reset x; bump x}
proc peek {name} {upvar 1 $name v; return $v}
proc ensure {name} {upvar 1 $name v; if {![info exists v]} {set v 0}}
proc maybe {name c} {upvar 1 $name v; if {$c} {set v 1}}
set r 5; renew r; set p 3; peek p; ensure e; maybe m 0; maybe k 1
puts "$r $p [info exists e] [info exists m] [info exists k]"
