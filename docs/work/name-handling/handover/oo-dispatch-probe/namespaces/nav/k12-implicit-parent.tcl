namespace eval a::b { proc p {} {return ab} }
puts [a::b::p] ;# CALL p
