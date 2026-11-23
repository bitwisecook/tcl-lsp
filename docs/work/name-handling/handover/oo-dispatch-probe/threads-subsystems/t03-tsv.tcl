# tsv:: shared variables are visible across threads.
lappend auto_path /opt/google-cloud-sdk/platform/bundledpythonunix/lib
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
