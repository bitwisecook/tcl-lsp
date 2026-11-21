package require overrides
proc ::greet {} {return orig}
namespace eval ::app { proc run {} { overrides::withCmdOverrides {greet {lambda {{} {return over}}}} { greet } } }
puts [catch ::app::run m]:$m
puts after:[greet]
