namespace eval ::lib { proc helper {} { return h } }
puts [apply {{} {helper} ::lib}]
puts [catch {apply {{} {helper}}} m]:$m ;# BAD helper not in ::
