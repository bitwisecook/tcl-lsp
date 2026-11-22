proc p {} { upvar #0 ::g alias; set alias 5 }
p
puts $g
