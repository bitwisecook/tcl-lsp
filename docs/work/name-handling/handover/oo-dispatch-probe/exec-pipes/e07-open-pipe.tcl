# open "|cmd" and open |[list ...]: command pipelines as channels.
set ch [open "|echo piped" r]
puts "read=[gets $ch]"
puts "pid=[llength [pid $ch]]"
close $ch
set ch [open |[list cat] r+]
puts $ch "round trip"
flush $ch
chan close $ch write
puts "rw=[gets $ch]"
close $ch
set ch [open |[list false] r]
read $ch
puts "close-status=[catch {close $ch} m] [lindex $::errorCode 0]"
set ch [open "|cat no-such-file" r]
read $ch
puts "close-stderr=[catch {close $ch} m] [string match *no-such-file* $m]"
puts "bad-open=[catch {open |[list no-such-program-xyz] r} m] [string match *no-such* $m]"
