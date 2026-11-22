proc double {n} { return [expr {$n * 2}] }
proc p {} { return [double 21] }
puts [p]
