proc cb2 {args} { set ::z 7 }
trace add variable w write cb2
set w 1
puts "z=1"
