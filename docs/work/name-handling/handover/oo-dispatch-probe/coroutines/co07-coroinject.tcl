# coroinject and coroprobe (Tcl 8.7/9.0 and later).
proc body {} { set v 10; set r [yield first]; return "r=$r v=$v" }
coroutine c body
puts [list [catch {coroprobe c set v} m] $m]
proc inj {args} { return "injected:$args" }
puts [list [catch {coroinject c inj x} m] $m]
puts [c resumed]
