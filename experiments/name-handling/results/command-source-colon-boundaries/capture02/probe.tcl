puts [list provider [info patchlevel] executable [info nameofexecutable] library [info library]]
set loaded [open [file join [info library] init.tcl] rb]
set initBytes [read $loaded]
close $loaded
set saved [open [file join [file dirname [info script]] [info patchlevel].loaded-init.tcl] wb]
puts -nonewline $saved $initBytes
close $saved
namespace eval ::a {}
set x {}
proc a:::$x:b {} {return HIT}
set code [catch {::a::b} result]
puts [list Q-COLON-WILDCARD-BOUNDARY-01 code $code result $result selected [namespace which -command ::a::b]]
set code [catch {::a::OTHER} result]
puts [list Q-COLON-WILDCARD-BOUNDARY-01-NEGATIVE code $code result $result selected [namespace which -command ::a::OTHER]]
