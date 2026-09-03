# T2: the list core. Every op here is a registry intrinsic candidate.
set l {}
lappend l 3 1 2
lappend l 5
puts [llength $l]
puts [lindex $l 1]
puts [lindex $l end]
puts [lrange $l 1 2]
puts [lsort -integer $l]
puts [lsearch $l 2]
puts [join $l ,]
puts [lreverse $l]
lset l 0 9
puts $l
puts [linsert $l 1 x]
puts [lreplace $l 0 1]
puts [concat $l {a b}]
lassign $l p q r
puts "$p $q $r"
