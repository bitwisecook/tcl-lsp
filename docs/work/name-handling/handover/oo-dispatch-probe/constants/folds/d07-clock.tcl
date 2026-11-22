proc p {} { return [expr {[clock seconds] > 0}] }
puts [p]
