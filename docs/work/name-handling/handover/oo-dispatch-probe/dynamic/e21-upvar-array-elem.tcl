set a(1) x
proc p {} { upvar #0 a(1) e; set e y }
p
puts $a(1)
