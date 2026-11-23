# exec runs a program directly (no shell); trailing newline stripped unless -keepnewline.
puts "echo=[exec echo hello world]"
puts "keep=[string length [exec -keepnewline echo ab]]"
puts "strip=[string length [exec echo ab]]"
puts "true=[catch {exec true} m] <$m>"
set rc [catch {exec false} m opts]
puts "false=$rc code=[lindex [dict get $opts -errorcode] 0] status=[lindex [dict get $opts -errorcode] 2] msg=<$m>"
puts "missing=[catch {exec no-such-program-xyz} m] [lindex $::errorCode 0] [lindex $::errorCode 1]"
puts "dashdash=[exec -- echo -keepnewline]"
