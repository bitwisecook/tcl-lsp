proc f {} {set original [expr {3.5}]; upvar 0 original view; return [expr {$view}]}
proc g {} {return [expr {7 << 1}]}
proc h {} {set container [list [expr {7 << 1}]]; return [lindex $container 0]}
foreach command {f g h} {
    set code [catch $command result]
    set representation unavailable
    catch {set representation [::tcl::unsupported::representation $result]}
    puts [list $command $code $result $representation]
}
