# lsearch matching modes
set l {apple banana {a*} Apple cherry a.b}
puts [lsearch $l a*]
puts [lsearch -all $l a*]
puts [lsearch -exact $l a*]
puts [lsearch -regexp -all -inline $l {^a.}]
puts [lsearch -glob -nocase -all -inline $l a*]
puts [lsearch -not -all -inline $l a*]
puts [lsearch -regexp $l {(}]
