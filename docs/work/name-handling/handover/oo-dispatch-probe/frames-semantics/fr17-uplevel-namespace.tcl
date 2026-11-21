namespace eval ::n { proc p {} { uplevel 1 {namespace current} } }
proc q {} { namespace eval ::m { ::n::p } }
namespace eval ::m {}
puts [q]:[::n::p]
