proc scope_callback {} {
    set own BEFORE
    set code [catch {uplevel #1 {set x CALLBACK}} message]
    return [list $code $message]
}
proc current_local {script} {
    set x LOCAL
    set code [catch {uplevel #0 $script} message]
    return [list $code $message $x]
}
foreach script {
    {set x GLOBAL}
    {uplevel #1 {set x ABSOLUTE}}
    {uplevel 1 {set x RELATIVE}}
    {scope_callback}
} {
    puts [list scope $script [current_local $script]]
}
set x GLOBAL
set trace_status UNCALLED
proc scope_trace {name element operation} {
    global trace_status
    set code [catch {uplevel #1 {set x TRACECALLBACK}} message]
    set trace_status [list $code $message]
}
trace variable x w scope_trace
puts [list trace [current_local {set x TRIGGER}] $trace_status]
trace vdelete x w scope_trace
proc make_namespace_alias {} {
    set x LOCAL
    set code [catch {upvar 0 x ::scope_alias} message]
    return [list $code $message $x]
}
puts [list global_alias [make_namespace_alias]]
