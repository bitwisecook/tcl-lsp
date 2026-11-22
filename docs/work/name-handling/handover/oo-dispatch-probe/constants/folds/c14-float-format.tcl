proc p {} { return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {2**64}] [expr {-0.0}]] }
puts [p]
