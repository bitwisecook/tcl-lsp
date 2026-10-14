proc current_local {script} {
    set x LOCAL
    set code [catch {uplevel #0 $script} message]
    return [list $code $message $x]
}
set x GLOBAL
set trace_status UNCALLED
proc scope_trace {name element operation} {
    global trace_status
    set code [catch {uplevel #1 {set x TRACECALLBACK}} message]
    set trace_status [list $code $message]
}
set install [catch {trace add variable x write scope_trace} install_message]
set legacy 0
if {$install} {
    set legacy 1
    set install [catch {trace variable x w scope_trace} install_message]
}
puts [list install $install $install_message $legacy]
if {!$install} {
puts [list trace [current_local {set x TRIGGER}] $trace_status]
if {$legacy} {trace vdelete x w scope_trace} else {trace remove variable x write scope_trace}
}
