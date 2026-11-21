namespace eval ::cfg {}
proc p {} { namespace upvar ::cfg missing l; return [info exists l] }
puts [p]
proc q {} { namespace upvar ::nons x l } 
puts [catch q m]:$m
