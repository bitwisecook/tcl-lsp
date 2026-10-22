namespace eval ns {}
set ::ns::caf(i) ascii
set ::ns::café(i) uni
set index i
puts [catch {puts $::ns::café($index)} m]
puts $m
