# Timing-sensitive variant kept to document the debug VM's slow turn: a 20 ms timer becomes due during update.
set t0 [clock milliseconds]
after 0 {lappend ::o a0}
after 20 {lappend ::o late}
update
puts $o
