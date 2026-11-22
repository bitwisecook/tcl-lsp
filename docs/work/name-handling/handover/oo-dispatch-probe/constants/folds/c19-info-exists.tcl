proc p {} { set x 1; if {[info exists x]} { return yes }; return no }
puts [p]
