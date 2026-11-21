proc p {} { set loc ploc; apply {{} { list [info level] [uplevel 1 {set loc}] [namespace current] } ::n} }
namespace eval ::n {}
puts [p]
