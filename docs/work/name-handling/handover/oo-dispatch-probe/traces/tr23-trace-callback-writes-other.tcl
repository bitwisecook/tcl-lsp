# A variable-trace callback writes a different global; a later read must see it.
set z 1
proc cb2 {args} { set ::z 7 }
trace add variable w write cb2
set w 1
puts "z=$z"
