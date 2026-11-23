# a channel handle is an ordinary value
set d [file join [pwd] t11]
set f [open $d w]
set handles [list $f]
dict set state out $f
proc emit {ch msg} { puts $ch $msg }
emit [lindex $handles 0] one
emit [dict get $state out] two
puts "is channel name: [string match file* $f]"
puts "in chan names: [expr {[lsearch -exact [chan names] $f] >= 0}]"
set g $f
close $g
puts "after close: [catch {puts $f three} m] $m"
puts [regsub {file\w+} $m CHAN]
set f [open $d r]; set all [read $f]; close $f
puts [list content $all]
file delete $d
