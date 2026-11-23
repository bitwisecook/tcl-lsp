# catch returns the completion code of its body: 0 ok, 1 error, 2 return, 3 break, 4 continue, custom.
proc show {script} { set c [catch $script r]; puts "[list $script] -> $c [list $r]" }
show {set x 1}
show {error boom}
show {return val}
show {break}
show {continue}
show {return -code 5 five}
show {return -code 1 viaReturn}
show {return -code -1 neg}
