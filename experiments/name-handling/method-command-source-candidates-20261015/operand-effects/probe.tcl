puts [list PATCHLEVEL [info patchlevel]]
proc helper {args} {return OLD}
set code [catch {::helper {[replaceHelper]}} result]
puts [list LITERAL_ARGUMENT $code $result]
proc replaceHelper {} {
    rename ::helper {}
    proc ::helper {args} {return NEW}
    return payload
}
set code [catch {::helper [replaceHelper]} result]
puts [list COMMAND_ARGUMENT $code $result]
