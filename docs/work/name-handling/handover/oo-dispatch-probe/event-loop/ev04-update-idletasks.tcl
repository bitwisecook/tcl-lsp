# update idletasks runs idle callbacks but not timers; update runs due timers only.
after idle {set ::i 1}
after 0 {set ::t 1}
update idletasks
puts [list [info exists i] [info exists t]]
after 500 {set ::late 1}
update
puts [list [info exists t] [info exists late]]
