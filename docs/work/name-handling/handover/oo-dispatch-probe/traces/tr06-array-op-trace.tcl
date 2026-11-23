# 'array' operation traces fire for array subcommands.
proc cb {n1 n2 op} { puts "cb {$n1} {$n2} $op" }
array set a {k 1}
trace add variable a array cb
puts [array size a]
array names a
array set a {z 2}
