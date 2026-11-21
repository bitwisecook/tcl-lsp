namespace eval ::d { proc p {} {return old} }
namespace delete ::d
namespace eval ::d { proc p {} {return new} }
puts [::d::p] ;# CALL p
