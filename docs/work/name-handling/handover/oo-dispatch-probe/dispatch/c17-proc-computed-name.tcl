foreach n {one two} { proc $n {} [list return $n] }
puts [one][two]
