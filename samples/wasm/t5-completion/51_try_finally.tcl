# T5: try/on/trap/finally ordering.
set log {}
proc t {x} {
    global log
    try {
        if {$x == 1} { error boom {} {APP FAIL} }
        if {$x == 2} { return early }
        lappend log body$x
    } trap {APP} {msg} {
        lappend log "trap:$msg"
    } on error {msg} {
        lappend log "error:$msg"
    } finally {
        lappend log finally$x
    }
    return normal$x
}
puts [t 0]
puts [t 1]
puts [t 2]
puts $log
