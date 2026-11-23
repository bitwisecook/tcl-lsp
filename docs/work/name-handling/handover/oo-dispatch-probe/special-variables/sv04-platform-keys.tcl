# tcl_platform keys (host-independent part).
puts [lsort [array names tcl_platform]]
puts "platform=$tcl_platform(platform) engine=[expr {[info exists tcl_platform(engine)] ? $tcl_platform(engine) : {<none>}}]"
