namespace eval ::lib { proc helper {x} { return h$x } }
set cb [namespace eval ::lib {namespace code {helper 1}}]
puts [eval $cb]
puts [namespace inscope ::lib helper 2]
set bad [namespace code {helper 3}]
puts [catch {eval $bad} m]:$m ;# BAD captured in ::
