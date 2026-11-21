package require overrides
proc ::foo {} {return orig}
set n 0
puts [catch { foreach i {1 2 3} { overrides::withCmdOverrides {::foo {lambda {{} {return over}}}} { incr n; break } } } m]:$m:n=$n:[foo]
