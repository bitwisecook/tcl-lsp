proc p {} { return [expr {wide(9223372036854775807) + 1}] }
puts [p]
