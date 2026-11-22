proc p {} { set x 3; set cb [list expr {$x * 2}]; return [{*}$cb] }
puts [catch p m]:$m
