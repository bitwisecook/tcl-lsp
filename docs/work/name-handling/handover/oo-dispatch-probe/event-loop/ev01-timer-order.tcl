# Timer and idle ordering: due timers before idle callbacks; equal delays in registration order.
# Delays are wide (200/400 ms) because the debug VM takes ~35 ms per event-loop turn.
set o {}
after 200 {lappend ::o t200}
after idle {lappend ::o idle}
after 0 {lappend ::o t0a}
after 0 {lappend ::o t0b}
after 400 {set ::done 1}
vwait done
puts $o
