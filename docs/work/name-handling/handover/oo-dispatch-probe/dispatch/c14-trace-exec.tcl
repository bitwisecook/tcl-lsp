proc f {a} { return $a }
proc log args {}
trace add execution f enter log
puts [f 1]
