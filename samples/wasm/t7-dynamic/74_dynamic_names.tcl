# T7: computed variable names and info introspection - defeats static cells.
set prefix var
for {set i 0} {$i < 3} {incr i} { set ${prefix}$i [expr {$i * 10}] }
puts $var2
puts [set ${prefix}1]
puts [lsort [info vars var*]]
proc shows {} { info level 0 }
puts [shows]
puts [info exists var9]
