# Captured Jim alias wrappers resolve their selected logical level on each access.
proc maker {} {set a KEEP;upvar 0 a x;proc p {} {&x} {set x}}
maker
puts [list retired-read [catch p r] $r]
proc unrelated {} {set a OTHER;list [catch p r] $r}
puts [list unrelated-frame [unrelated]]
proc maker2 {} {set a OLD;upvar 0 a x;proc p2 {} {&x} {set x MODIFIED};proc r2 {} {&x} {set x}}
maker2
puts [list later-write-read [p2] [catch r2 r] $r]
