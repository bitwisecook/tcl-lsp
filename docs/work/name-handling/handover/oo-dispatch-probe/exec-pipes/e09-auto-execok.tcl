# auto_execok searches PATH; returns {} when missing. unknown auto-execs at top level only interactively.
puts "echo=[expr {[auto_execok echo] ne {}}]"
puts "abs=[string match /* [lindex [auto_execok cat] 0]]"
puts "missing=<[auto_execok no-such-program-xyz]>"
puts "unknown-no-autoexec=[catch {no-such-program-xyz} m] $m"
puts "echo-cmd=[catch {echo hi} m] $m"
