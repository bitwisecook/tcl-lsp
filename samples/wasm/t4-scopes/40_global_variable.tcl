# T4: global/variable link the proc frame to outer cells - the frame needs
# named cells for those names only.
set counter 0
namespace eval ::cfg { variable level 3 }
proc bump {} {
    global counter
    incr counter
}
proc level {} {
    variable ::cfg::level
    return $level
}
bump; bump
puts $counter
puts [level]
