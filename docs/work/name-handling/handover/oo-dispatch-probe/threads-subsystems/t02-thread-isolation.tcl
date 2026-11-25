# Each thread has its own interpreter: the creator's procs and globals are
# not visible to scripts sent to it.
if {[info exists ::env(TCL_LSP_PROBE_PACKAGES)]} {lappend auto_path $::env(TCL_LSP_PROBE_PACKAGES)}
if {[catch {package require Thread} v]} { puts "no-thread: $v"; return }
set shared 1
proc local {} { return local }
set t [thread::create]
puts "var=[catch {thread::send $t {set shared}} m] $m"
puts "proc=[catch {thread::send $t {local}} m] $m"
thread::send $t { proc remote {} { return remote } }
puts "remote=[thread::send $t remote]"
puts "remote-here=[catch {remote} m] $m"
thread::release $t
