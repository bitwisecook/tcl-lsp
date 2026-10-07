set events {}; set v OLD
proc watch {n k op} {lappend ::events $op; if {$op eq "read"} {uplevel 1 {unset v; set v NEW}}}
trace add variable v {read unset} watch
set h lappend
set answer [$h v EXTRA]
set summary [list $answer [set v] $events [trace info variable v]]
puts $summary
set summary
