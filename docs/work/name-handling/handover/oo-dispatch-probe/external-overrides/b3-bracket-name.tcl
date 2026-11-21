package require overrides
proc {x[puts INJECTED]} {} {return orig}
puts [catch {overrides::withCmdOverrides {{x[puts INJECTED]} {lambda {{} {return over}}}} {puts call:[{x[puts INJECTED]}]}} m]:$m
