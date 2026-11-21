proc setter {name} { upvar 1 $name v; set v aliased; info level }
interp alias {} al {} setter
proc p {} { set lv [al x]; return $x:$lv }
puts [p]
