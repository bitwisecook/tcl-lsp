set ::count 0
proc target {args} {return OLD}
proc change {} {
    incr ::count
    rename target saved
    proc target {args} {return NEW}
    return arg
}
puts [target [change]]
puts $::count
