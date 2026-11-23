puts [expr {1 +\
  2}]
puts [catch {expr {1 + 2 # trailing
}} r]:$r
