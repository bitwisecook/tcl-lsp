# Inside namespace eval an unqualified read falls back to the global.
namespace eval ns { puts "ns=$tcl_version" }
namespace eval ns { set auto_path [linsert $auto_path 0 /nowhere] }
puts "global-changed=[expr {[lindex $auto_path 0] eq {/nowhere}}] ns-var=[info exists ns::auto_path]"
