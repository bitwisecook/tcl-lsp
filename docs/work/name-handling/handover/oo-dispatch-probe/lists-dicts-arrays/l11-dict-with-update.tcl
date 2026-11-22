set d {a 1 b 2}
dict with d { set a 10; set c 3 }
puts $d
dict update d a A b B { incr A; unset B }
puts $d
