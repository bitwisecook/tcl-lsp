proc p {} { upvar 1 v alias; unset alias; set alias recreated }
proc q {} { set v orig; p; return $v }
puts [q]
