catch {set x ok} a(key)
puts [list [info exists a] [info exists a(key)] [set a(key)]]
proc p {} {catch {set x local} a(key); return [set a(key)]}
puts [p]
namespace eval ns {}
catch {set x qualified} ::ns::a(key)
puts [set ::ns::a(key)]
