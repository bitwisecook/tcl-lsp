puts [list VERSION [info patchlevel]]
set available [expr {[llength [info commands lassign]] != 0}]
puts [list AVAILABLE $available]
if {$available} {
    if {[llength [info commands alias]]} {
        alias la lassign
        puts {ALIAS JimAlias}
    } else {
        interp alias {} la {} lassign
        puts {ALIAS TclInterpAlias}
    }
    proc probe {mode initial} {
        unset -nocomplain first old one
        if {$initial} {set first old}
        if {$mode eq "inline"} {
            set code [catch {lassign {one two} first [set first]} result]
        } else {
            set code [catch {la {one two} first [set first]} result]
        }
        set report [list CASE $mode $initial CODE $code RESULT $result]
        foreach name {first old one} {
            set exists [info exists $name]
            lappend report $name $exists
            if {$exists} {lappend report [set $name]} else {lappend report {}}
        }
        puts $report
    }
    probe inline 0
    probe inline 1
    probe generic 0
    probe generic 1
} else {
    foreach mode {inline generic} {
        foreach initial {0 1} {puts [list UNAVAILABLE $mode $initial lassign]}
    }
}
