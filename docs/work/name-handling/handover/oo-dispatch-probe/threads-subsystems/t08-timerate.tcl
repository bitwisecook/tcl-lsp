# timerate: tcl::unsupported::timerate in 8.6.10+, ::timerate in 9.0.
puts "unsupported=[llength [info commands ::tcl::unsupported::timerate]] core=[llength [info commands ::timerate]]"
set cmd [expr {[llength [info commands ::timerate]] ? "timerate" : "tcl::unsupported::timerate"}]
if {[catch {$cmd {expr {1+1}} 20} r]} { puts "err=$r" } else { puts "shape=[llength $r]" }
