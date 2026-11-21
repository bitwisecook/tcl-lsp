package require overrides
proc {::a b} {} {return orig}
puts [catch {overrides::withCmdOverrides {{::a b} {lambda {{} {return over}}}} {{::a b}}} m]:$m
