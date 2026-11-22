proc k {} { return 7 }
proc p {} { return [expr {[k] * 2}] }
puts [p]
