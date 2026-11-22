set r {}; lappend r 1;lappend r 2 ; lappend r "3;4"; lappend r {5;6}
puts [list $r [llength $r]]
