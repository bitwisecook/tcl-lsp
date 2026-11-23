# Jim dict sugar: $d(key) reads a dict-valued scalar
set d {a 1 b 2}
puts $d(a)
set d(c) 3
puts $d
puts [info exists d(z)]
