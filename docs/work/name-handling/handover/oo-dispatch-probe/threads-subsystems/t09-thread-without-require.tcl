# Without package require, thread:: commands do not exist in tclsh.
puts "thread-create=[llength [info commands ::thread::create]]"
puts "tsv-set=[llength [info commands ::tsv::set]]"
puts "call=[catch {thread::id} m] $m"
