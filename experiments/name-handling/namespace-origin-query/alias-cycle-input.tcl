puts [list IDENTITY [info patchlevel]]
if {[llength [info commands alias]]} {
    set a [catch {alias ::__no_cycle_a ::__no_cycle_b} av]
    set b [catch {alias ::__no_cycle_b ::__no_cycle_a} bv]
} else {
    set a [catch {interp alias {} ::__no_cycle_a {} ::__no_cycle_b} av]
    set b [catch {interp alias {} ::__no_cycle_b {} ::__no_cycle_a} bv]
}
puts [list SETUP first $a $av second $b $bv]
flush stdout
set code [catch {namespace origin ::__no_cycle_a} value]
puts [list ORIGIN $code $value]
