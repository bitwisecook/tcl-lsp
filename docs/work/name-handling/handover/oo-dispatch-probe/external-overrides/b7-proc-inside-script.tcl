package require overrides
proc ::foo {} {return orig}
puts [catch {overrides::withCmdOverrides {::foo {lambda {{} {return over}}}} { proc helper {} {return h}; helper }} m]:$m
