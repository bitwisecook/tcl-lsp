proc p {k} {set a(x) 1; if {[info exists a($k)]} {puts yes} else {puts no}}
p x
p y
