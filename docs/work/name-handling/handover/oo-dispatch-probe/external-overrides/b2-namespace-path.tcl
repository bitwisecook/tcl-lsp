package require overrides
namespace eval ::lib { proc greet {} {return orig} }
namespace eval ::app { namespace path ::lib; proc run {} { overrides::withCmdOverrides {greet {lambda {{} {return over}}}} { greet } } }
puts [catch ::app::run m]:$m
