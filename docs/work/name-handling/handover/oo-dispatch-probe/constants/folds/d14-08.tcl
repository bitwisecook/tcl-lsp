proc p {} { set x 08; return [string is integer $x] }
puts [p]
