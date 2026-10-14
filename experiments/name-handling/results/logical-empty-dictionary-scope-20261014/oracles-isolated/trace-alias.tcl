puts [list VERSION [info patchlevel]]
set observer_calls 0
proc observer {args} {incr ::observer_calls}
set code [catch {trace add variable observed write observer} result]
puts [list TRACE_SETUP $code $result]
if {$code == 0} {set d {observed 14}; dict with d {}; puts [list RESULT [set d] [set observed] $observer_calls]}
