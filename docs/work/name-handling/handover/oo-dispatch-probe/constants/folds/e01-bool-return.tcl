proc yes {} { return true }
proc p {} { return [yes] }
puts [p]
puts [string length [yes]]
