# Output on stderr makes exec fail unless -ignorestderr or 2> redirects it;
# a non-zero exit still fails.
puts "stderr-fails=[catch {exec cat no-such-file} m] [lindex $::errorCode 0]"
set ch [open sink.txt w]
puts "ignorestderr=[catch {exec -ignorestderr cat no-such-file 2>@ $ch} m] [lindex $::errorCode 0]"
close $ch
puts "ignorestderr-zero=[catch {exec -ignorestderr echo fine} m] $m"
puts "unknown-switch=[catch {exec -bogus echo x} m] $m"
file delete sink.txt
