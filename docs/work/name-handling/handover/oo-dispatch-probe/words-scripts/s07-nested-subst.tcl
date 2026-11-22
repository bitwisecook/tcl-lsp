set v 2
puts [list [expr {[llength "a [list b c] d"]}] "[string repeat x $v]" {[not subst]}]
