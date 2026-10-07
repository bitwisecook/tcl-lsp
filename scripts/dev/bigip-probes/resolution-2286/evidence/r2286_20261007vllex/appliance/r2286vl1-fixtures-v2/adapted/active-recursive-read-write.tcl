set events {}; set v OLD
proc watch {n k op} {upvar 1 events events; lappend events $op; if {$op eq "read"} {uplevel 1 {set v INNER; set v}}}
trace add variable v {read write} watch
set h lappend
set answer [$h v EXTRA]
trace remove variable v {read write} watch
set summary [list $answer [set v] $events]
set summary
