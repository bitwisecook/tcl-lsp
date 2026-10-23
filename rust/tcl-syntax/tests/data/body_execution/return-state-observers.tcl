# Error globals are observable native state; ordinary return does not write them.
set events {}
proc observed {args} {global events; lappend events [lindex $args 0]}
if {[package vcompare [info tclversion] 8.5] < 0} {
    trace variable ::errorCode w observed
    trace variable ::errorInfo w observed
} else {
    trace add variable ::errorCode write observed
    trace add variable ::errorInfo write observed
}
proc probe {label body} {
    global events
    set events {}
    proc p {} $body
    set code [catch {p} result]
    puts [list $label $code $result $events]
}
probe plain {return VALUE}
probe error {return -code error -errorcode {CUSTOM VALUE} -errorinfo CUSTOMINFO VALUE}
probe explicit_ok {return -code ok -errorcode {CUSTOM VALUE} -errorinfo CUSTOMINFO VALUE}
probe invalid {return -code invalid VALUE}
if {[package vcompare [info tclversion] 8.5] >= 0} {
    probe error_level0 {return -level 0 -code error -errorcode {CUSTOM VALUE} -errorinfo CUSTOMINFO VALUE}
    probe error_options {return -options {-code 1 -level 0 -errorcode {CUSTOM VALUE} -errorinfo CUSTOMINFO} VALUE}
}
