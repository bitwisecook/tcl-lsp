proc p {} {return p}
rename p q
puts [list [q] [catch p m] $m [llength [info commands p]]]
proc p {} {return newp}
puts [list [p] [q]]
