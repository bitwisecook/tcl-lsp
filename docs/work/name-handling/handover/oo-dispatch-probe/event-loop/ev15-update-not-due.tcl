# update does not wait for a timer that is not yet due.
after 200 {set ::late 1}
set t0 [clock milliseconds]
update
puts [list [info exists late] [expr {[clock milliseconds] - $t0 < 150}]]
