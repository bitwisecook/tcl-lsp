namespace eval ::a { proc p {} {return p} }
namespace eval ::b {}
rename ::a::p ::b::q
puts [::b::q] ;# CALL q
