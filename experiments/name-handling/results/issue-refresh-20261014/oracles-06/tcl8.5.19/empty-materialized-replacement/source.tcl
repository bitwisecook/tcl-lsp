rename set original_set
proc set args {return CUSTOM}
puts [eval {set {} VALUE}]
puts [info exists {}]
