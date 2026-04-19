# ``switch -glob`` must glob-match each pattern against the subject.
# Previously the CFG converted the whole switch into an IRBarrier
# (routed to tcl_eval), but the runtime interpreter has no ``switch``
# command, so every -glob switch trapped with ``unknown command: switch``.
set results {}
switch -glob hello {foo {lappend results A} h* {lappend results B}}
if {$results ne "B"} { error "glob h* missed: $results" }

set results {}
switch -glob foobar {f*bar {lappend results M} default {lappend results D}}
if {$results ne "M"} { error "glob middle-wildcard missed" }

set results {}
switch -glob zed {foo {lappend results A} default {lappend results D}}
if {$results ne "D"} { error "glob default missed" }

# Exact still works alongside the new glob path.
set results {}
switch -- exact {foo {lappend results NO} exact {lappend results YES}}
if {$results ne "YES"} { error "exact broken" }
return ok
