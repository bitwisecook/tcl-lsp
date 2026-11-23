# tcl::process (8.7/9.0): track background children started by exec &.
if {[catch {tcl::process list} m]} { puts "no-tcl-process: $m"; return }
puts "autopurge=[tcl::process autopurge]"
tcl::process autopurge 0
set pid [exec true &]
after 300
puts "listed=[expr {$pid in [tcl::process list]}]"
set st [tcl::process status -wait $pid]
puts "status=$st"
tcl::process purge $pid
puts "purged=[expr {$pid in [tcl::process list]}]"
set pid2 [exec false &]
puts "fail=[lindex [tcl::process status -wait $pid2] 1 0]"
tcl::process autopurge 1
