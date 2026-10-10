puts [list version [info patchlevel]]
proc r2286_observe {label script} {
    set status [catch {uplevel #0 $script} result]
    if {$status} {set code $::errorCode} else {set code {}}
    puts [list observation $label status $status result $result errorCode $code]
}
r2286_observe create {interp create s}
r2286_observe parent-command {info commands s}
r2286_observe handle-return {s eval {return ok}}
r2286_observe path-return {interp eval s {return ok}}
r2286_observe handle-value {s eval {set value ok}}
r2286_observe path-value {interp eval s {set value ok}}
r2286_observe delete {interp delete s}
r2286_observe parent-command-after-delete {info commands s}
