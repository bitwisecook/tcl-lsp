set {} ORIGINAL
proc remote {} {upvar #0 {} local; set local REMOTE}
remote
puts [list remote [set {}]]
set target BEFORE
proc local {} {upvar #0 target {}; set {} LOCAL}
local
puts [list local $target]
proc both {} {upvar #0 {} {}; set {} BOTH}
both
puts [list both [set {}]]
proc dynamic {name} {upvar #0 {} $name; set $name DYNAMIC}
dynamic chosen
puts [list dynamic [set {}]]
