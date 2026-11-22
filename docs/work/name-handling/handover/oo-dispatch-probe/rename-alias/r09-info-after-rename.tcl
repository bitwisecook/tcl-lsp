proc p {x} {return $x}
rename p q
puts [list [info procs q] [info args q] [info body q] [info procs p]]
