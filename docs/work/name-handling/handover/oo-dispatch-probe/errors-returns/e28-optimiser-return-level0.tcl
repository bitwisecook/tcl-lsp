# return -level 0 completes with code ok at the current level: the next command runs.
proc v {} { return -level 0 lvl0 ; puts reached-v }
puts [v]
proc w {} { return -level 0 -code ok x ; return after }
puts [w]
