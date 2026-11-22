proc p {} { set x 1; trace add variable x read {apply {{args} {uplevel 1 {set x 7}}}}; return [expr {$x + 0}] }
puts [p]
