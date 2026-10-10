# Exact ASCII source-input lookup experiment. No internal native object or argv inspection.
puts [list version [info patchlevel]]
puts [list trace-command [info commands trace]]
set ::events {}
namespace eval ::Dest {
    proc cb {tag args} {
        lappend ::events [list $tag Dest [llength $args] [uplevel 1 {namespace current}]]
    }
}
namespace eval ::A {
    proc cb {tag args} {
        lappend ::events [list $tag A [llength $args] [uplevel 1 {namespace current}]]
    }
}
namespace eval ::B {
    proc cb {tag args} {
        lappend ::events [list $tag B [llength $args] [uplevel 1 {namespace current}]]
    }
    proc variable_trigger {} {set ::watched VALUE}
    proc command_trigger {} {rename ::target ::moved}
    proc execution_trigger {} {::target}
}
proc ::target {} {return VALUE}
foreach head {cb ::Dest::cb} {
    foreach kind {variable command execution} {
        set ::events {}
        if {$kind == "variable"} {
            set operations write
            set name ::watched
        } elseif {$kind == "command"} {
            set operations rename
            set name ::target
        } else {
            set operations {enter leave}
            set name ::target
        }
        set prefix [list $head $kind]
        set addCode [catch {namespace eval ::A [list trace add $kind $name $operations $prefix]} addResult]
        if {$addCode} {
            puts [list add $kind $head $addCode $addResult]
            continue
        }
        set trigger [list ::B::${kind}_trigger]
        set fireCode [catch $trigger fireResult]
        puts [list fire $kind $head $fireCode $fireResult $::events]
        if {$kind == "command"} {
            catch {trace remove command ::moved $operations $prefix}
            rename ::moved ::target
        } else {
            catch [list trace remove $kind $name $operations $prefix]
        }
    }
}
