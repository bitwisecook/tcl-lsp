puts [list version [info patchlevel] dict_available [expr {[llength [info commands dict]] != 0}]]
if {[llength [info commands dict]] == 0} {exit 0}
proc missing_probe {name body} {
    catch {unset $name}
    set ::body_ran 0
    set code [catch {dict with $name $body} message]
    puts [list name $name body $body code $code message $message body_ran $::body_ran exists [info exists $name]]
}
foreach name [list {} {$missing}] {
    missing_probe $name {}
    missing_probe $name {set ::body_ran 1}
}
