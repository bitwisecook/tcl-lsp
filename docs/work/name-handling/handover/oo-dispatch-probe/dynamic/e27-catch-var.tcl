proc p {} { catch {set inside 1}; return $inside }
puts [p]
