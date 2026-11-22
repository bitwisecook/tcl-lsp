set g 1
proc p {} { return [expr {$::g + 1}] }
set g 5
puts [p]
