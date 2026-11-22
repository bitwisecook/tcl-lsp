array set a {x 1}
array default set a 0
puts [list $a(x) $a(nope) [array default get a] [array size a]]
