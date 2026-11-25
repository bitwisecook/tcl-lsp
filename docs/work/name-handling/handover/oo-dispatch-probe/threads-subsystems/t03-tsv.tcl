# tsv:: shared variables are visible across threads.
if {[info exists ::env(TCL_LSP_PROBE_PACKAGES)]} {lappend auto_path $::env(TCL_LSP_PROBE_PACKAGES)}
if {[catch {package require Thread} v]} { puts "no-thread: $v"; return }
tsv::set cfg name alpha
tsv::incr cfg hits 5
tsv::lappend cfg items a b
set t [thread::create]
puts "from-thread=[thread::send $t {tsv::get cfg name}]"
thread::send $t {tsv::incr cfg hits}
puts "hits=[tsv::get cfg hits]"
puts "items=[tsv::get cfg items]"
puts "names=[lsort [tsv::names cfg*]]"
puts "keys=[lsort [tsv::array names cfg]]"
puts "exists=[tsv::exists cfg nope]"
thread::release $t
