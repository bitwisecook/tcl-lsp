# vwait services idle callbacks while it waits.
after idle {set ::flag idle-ran}
after 100 {set ::flag timer-ran}
vwait flag
puts $flag
