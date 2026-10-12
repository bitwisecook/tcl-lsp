puts [list stock [expr {abs(-3)}]]
namespace eval ::tcl::mathfunc {}
proc ::tcl::mathfunc::abs {value} {return 99}
puts [list replaced [expr {abs(-3)}]]
