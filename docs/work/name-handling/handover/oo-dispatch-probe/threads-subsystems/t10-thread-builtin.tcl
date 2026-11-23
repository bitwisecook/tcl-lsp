# Same checks as t01-t03, but use thread:: commands if they already exist
# (the VM registers them natively) and only otherwise package require Thread.
if {![llength [info commands ::thread::create]]} {
  lappend auto_path /opt/google-cloud-sdk/platform/bundledpythonunix/lib
  if {[catch {package require Thread} v]} { puts "no-thread: $v"; return }
  puts "source=package"
} else { puts "source=builtin provide=[package provide Thread]" }
set shared 1
proc local {} { return local }
set t [thread::create {thread::wait}]
puts "send=[thread::send $t {expr {6 * 7}}]"
puts "var=[catch {thread::send $t {set shared}} m] $m"
puts "proc=[catch {thread::send $t {local}} m] $m"
thread::send -async $t {expr {1 + 1}} ::res
vwait ::res
puts "async=$::res"
tsv::set cfg name alpha
tsv::incr cfg hits 5
thread::send $t {tsv::incr cfg hits}
puts "hits=[tsv::get cfg hits] from-thread=[thread::send $t {tsv::get cfg name}]"
puts "errorcode=[catch {thread::send $t {error boom {} {MY CODE}}} m o] $m [dict get $o -errorcode]"
thread::release $t
