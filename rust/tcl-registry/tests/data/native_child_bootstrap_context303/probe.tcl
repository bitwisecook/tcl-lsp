puts [list version [info patchlevel]]
set is_jim [string match 0.* [info patchlevel]]
namespace eval jim {}
foreach name {argv argc argv0 jim::argv0 jim::exe jim::lineedit} {
    set $name "copy-$name"
}
if {$is_jim} {set child [interp]} else {set child [interp create]}
foreach command {namespace package interp array binary pack unpack class} {
    set script [list info commands $command]
    if {$is_jim} {set names [$child eval $script]} else {set names [interp eval $child $script]}
    puts [list child_command $command [expr {[llength $names] != 0}]]
}
foreach name {argv argc argv0 jim::argv0 jim::exe jim::lineedit} {
    if {$is_jim} {
        set exists [$child eval [list info exists $name]]
        if {$exists} {set value [$child eval [list set $name]]} else {set value ABSENT}
    } else {
        set exists [interp eval $child [list info exists $name]]
        if {$exists} {set value [interp eval $child [list set $name]]} else {set value ABSENT}
    }
    puts [list child_variable $name $exists $value]
}
if {$is_jim} {$child delete} else {interp delete $child}
