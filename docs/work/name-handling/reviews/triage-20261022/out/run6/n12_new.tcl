proc p {} {return initial}
#padding0123
rename p q
proc p {} {}
puts [llength [info commands ::p]]
