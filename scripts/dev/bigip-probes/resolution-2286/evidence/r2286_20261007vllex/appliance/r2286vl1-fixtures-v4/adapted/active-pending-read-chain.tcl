set events {}; set v OLD
proc later {n k op} {upvar 1 events events; lappend events later-$op}
proc kill {n k op} {upvar 1 events events; lappend events kill-$op; if {$op eq "read"} {uplevel 1 {unset v; set v NEW}}}
trace add variable v read later
trace add variable v {read unset} kill
set h lappend
set answer [$h v EXTRA]
set summary [list $answer [set v] $events [trace info variable v]]
set summary
