proc p {} { return [string length "é\U1F600"] }
puts [p]
