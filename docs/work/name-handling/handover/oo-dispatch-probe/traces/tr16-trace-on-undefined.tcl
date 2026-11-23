# Adding a trace to a missing variable keeps it non-existent until set.
proc cb {n1 n2 op} { puts "cb $op" }
trace add variable u write cb
puts "exists=[info exists u] vars=[lsearch -exact [info vars] u]"
set u 1
puts "exists=[info exists u]"
