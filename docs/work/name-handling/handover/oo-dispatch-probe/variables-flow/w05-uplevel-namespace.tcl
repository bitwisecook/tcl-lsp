proc p {} { uplevel #0 { set made 1 } }
p
puts $made
