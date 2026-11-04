set cases {
    {exact {proc p {} {
    foreach x {} {}
    if {[info exists x]} { puts yes }
    puts $x
}
foreach y {} {}
puts $y
} {}}
    {without-guard {
proc p {} {foreach x {} {}; return $x}
error fail
    } {}}
    {guarded-only {
proc p {} {foreach x {} {}; if {[info exists x]} {return $x}; return quiet}
error fail
    } {}}
    {guarded-later-read {
proc p {} {foreach x {} {}; if {[info exists x]} {set other 1}; return $x}
error fail
    } {}}
    {nonempty-loop {
proc p {} {foreach x {7} {}; return $x}
error fail
    } {}}
    {initialised-before-empty-loop {
proc p {} {set x 7; foreach x {} {}; return $x}
error fail
    } {}}
    {formal-before-empty-loop {
proc p {x} {foreach x {} {}; return $x}
error fail
    } {7}}
    {mutation-after-guard {
proc p {} {set x 7; if {[info exists x]} {unset x}; foreach x {} {}; return $x}
error fail
    } {}}
    {by-name-writer {
proc operation {name} {upvar 1 $name cell; set cell 7}
proc p {} {operation x; foreach x {} {}; return $x}
error fail
    } {}}
}
set ordinal 0
foreach case $cases {
    set id [lindex $case 0]
    set script [lindex $case 1]
    set args [lindex $case 2]
    if {$id eq "exact"} {
        binary scan $script H* sourceHex
        puts [list source-bytes exact $sourceHex]
    }
    set ns ::case[incr ordinal]
    set loadRc [catch {namespace eval $ns $script} loadResult]
    set call [linsert $args 0 ${ns}::p]
    set callRc [catch {eval $call} callResult]
    puts [list $id $loadRc $loadResult $callRc $callResult]
}
