foreach member {level frame vars locals coroutine errorstack body args default commands procs class functions cmdtype patchlevel tclversion nameofexecutable sharedlibextension library hostname script cmdcount complete cancel loaded object constant constants globals exists totally_made_up} {
 set code [catch [list info $member] result]
 puts [list $member $code $result]
}
