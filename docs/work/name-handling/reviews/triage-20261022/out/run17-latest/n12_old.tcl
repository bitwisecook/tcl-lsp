proc p {} {return initial}
proc p {} {}
rename p q
#padding0123
puts [llength [info commands ::p]]
