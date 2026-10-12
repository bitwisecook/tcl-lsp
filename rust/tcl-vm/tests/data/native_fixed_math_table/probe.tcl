puts [list abs [expr {abs(-4)}] pow [expr {pow(2,3)}] randRange [expr {rand() >= 0 && rand() < 1}]]
set touched 0
set code [catch {expr {abs([incr touched], 2)}} message]
puts [list arity $code $touched $message]
set touched 0
set code [catch {expr {missing_function([incr touched])}} message]
puts [list missing $code $touched $message]
namespace eval ::tcl::mathfunc {}
proc ::tcl::mathfunc::abs {value} {return COMMAND_SHADOW}
puts [list commandShadow [expr {abs(-4)}]]
