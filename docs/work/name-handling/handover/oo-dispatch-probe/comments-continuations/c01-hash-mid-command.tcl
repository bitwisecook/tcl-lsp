# '#' starts a comment only where a command could start
puts [catch {eval {set x 1 # not a comment}} r]:$r
set y 2 ;# this is a comment
puts y=$y
