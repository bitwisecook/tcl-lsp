proc p {} { list [catch {uplevel 5 {set a}} m1] $m1 [catch {upvar 3 x y} m2] $m2 [catch {uplevel #-1 {}} m3] $m3 }
puts [p]
puts [catch {uplevel 1 {set a}} m]:$m
