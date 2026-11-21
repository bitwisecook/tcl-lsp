package require overrides
namespace eval ::app { proc run {} { overrides::withCmdOverrides {clock {lambda {{args} {return fake}}}} { clock seconds } } }
puts [catch ::app::run m]:$m
puts clock-restored:[string is integer [clock seconds]]
