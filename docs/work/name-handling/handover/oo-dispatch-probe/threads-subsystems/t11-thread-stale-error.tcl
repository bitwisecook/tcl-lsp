# A caught error from a synchronous thread::send must not resurface later.
if {![llength [info commands ::thread::create]]} {
  if {[info exists ::env(TCL_LSP_PROBE_PACKAGES)]} {lappend auto_path $::env(TCL_LSP_PROBE_PACKAGES)}
  if {[catch {package require Thread} v]} { puts "no-thread: $v"; return }
}
set t [thread::create {thread::wait}]
puts "caught=[catch {thread::send $t {no-such-cmd}} m] $m"
puts "next-sync=[thread::send $t {expr {2 + 3}}]"
thread::send -async $t {expr {1 + 1}} ::res
vwait ::res
puts "async=$::res"
thread::release $t
