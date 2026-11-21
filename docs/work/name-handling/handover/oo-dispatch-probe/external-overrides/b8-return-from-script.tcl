package require overrides
proc ::foo {} {return orig}
proc p {} { overrides::withCmdOverrides {::foo {lambda {{} {return over}}}} { return early }; return late }
puts [p]:[foo]
