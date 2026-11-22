set l {apple banana cherry banana}
puts [list [lsearch $l banana] [lsearch -all $l banana] [lsearch -inline -glob $l c*] [lsearch -exact $l b*] [lsearch -regexp -all $l {^b|^c}] [lsearch -sorted {a b c d} c] [lsearch -not $l apple] [lsearch -index 1 {{a 3} {b 1}} 1] [lsearch -bisect -integer {1 3 5 7} 4]]
