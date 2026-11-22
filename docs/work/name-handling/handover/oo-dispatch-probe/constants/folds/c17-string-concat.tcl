proc p {} { set a foo; set b bar; set c "$a-$b"; return [string length $c] }
puts [p]
