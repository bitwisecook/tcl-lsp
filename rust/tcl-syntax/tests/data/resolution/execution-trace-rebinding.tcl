proc target {args} {return OLD}
proc cb {command operation} {
    rename target saved
    proc target {args} {return NEW}
    trace remove execution saved enter cb
}
trace add execution target enter cb
puts [target x]
puts [target x]
