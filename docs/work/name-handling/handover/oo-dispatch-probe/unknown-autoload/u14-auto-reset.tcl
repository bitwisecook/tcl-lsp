# auto_reset forgets auto_index and procs loaded through it.
set auto_index(rp) {proc rp {} {return rp1}}
puts [rp]
auto_reset
puts "after reset: [info exists auto_index(rp)] [info commands rp]"
