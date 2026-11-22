proc p {} { set s abc; append s def; return [string length $s] }
puts [p]
