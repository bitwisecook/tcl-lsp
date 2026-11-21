package require Tk
proc tick {n} { if {$n > 0} { after 10 [list tick [expr {$n - 1}]] } else { set ::done 1 } }
tick 3
vwait ::done
set id [after 1000 {puts never}]
after cancel $id
puts [catch {after 10 {tick}} m]:$m ;# BAD arity, raised later as a background error
after 50 {set ::w 1}; vwait ::w ;# the delayed tick raises a background error
puts ok
