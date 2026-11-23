# The finally clause sees variables the body set before it returned (W220 asks whether the assignment is read).
proc t {} { try { set a 1; return 1 } finally { puts "finally a=$a" } }
puts [t]
