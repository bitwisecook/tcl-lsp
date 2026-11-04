if {[llength [info commands trace]]} {
    set events 0
    proc writeObserver {name element operation} {incr ::events}
    if {[catch {trace add variable cell write writeObserver}]} {
        trace variable cell w writeObserver
    }
    foreach cell {} {error BODY}
    puts [list variable-observer-empty $events [info exists cell]]
    foreach cell {VALUE} {}
    puts [list variable-observer-nonempty $events $cell]
    catch {trace remove variable cell write writeObserver}
    catch {trace vdelete cell w writeObserver}
} else {
    puts [list variable-observer UNAVAILABLE]
}
if {[llength [info commands xtrace]]} {
    proc preserved {} {return ORIGINAL}
    proc commandObserver {type file line result command arguments} {
        if {$command eq "foreach"} {proc preserved {} {return OBSERVED}}
    }
    xtrace commandObserver
    foreach cell {} {error BODY}
    xtrace {}
    puts [list command-observer-empty [preserved] [info exists cell]]
    proc refusingObserver {type file line result command arguments} {
        if {$command eq "foreach"} {error OBSERVER_ERROR}
    }
    xtrace refusingObserver
    set code [catch {foreach cell {} {error BODY}} result]
    xtrace {}
    puts [list command-observer-error $code $result [info exists cell]]
} else {
    puts [list command-observer UNAVAILABLE]
}
