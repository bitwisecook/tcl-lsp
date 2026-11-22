proc p {} { set l {}; lappend l a b; return [llength $l] }
puts [p]
