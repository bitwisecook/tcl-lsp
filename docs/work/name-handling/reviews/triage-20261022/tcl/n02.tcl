namespace eval a {}
proc a::p  {} {return TWO}
proc a:::p {} {return THREE}
puts [list [a::p] [a:::p]]
