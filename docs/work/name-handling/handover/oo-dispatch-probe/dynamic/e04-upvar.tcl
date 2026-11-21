proc setit {name} { upvar 1 $name v; set v 5 }
proc use {} { setit q; return $q }
puts [use]
