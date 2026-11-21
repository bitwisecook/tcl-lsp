namespace eval ::w { proc cb {x} { return cb:$x } }
set c [namespace eval ::w {namespace code {cb}}]
puts [{*}$c 1]
