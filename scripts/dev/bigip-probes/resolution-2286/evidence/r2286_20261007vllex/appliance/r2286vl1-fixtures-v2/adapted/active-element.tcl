set events {}; set a(k) OLD
proc watch {n k op} {upvar 1 events events; lappend events $op; if {$op eq "read"} {uplevel 1 {unset a(k); set a(k) NEW}}}
trace add variable a(k) {read unset} watch
set h lappend
set answer [$h a(k) EXTRA]
set summary [list $answer [set a(k)] $events [trace info variable a(k)]]
set summary
