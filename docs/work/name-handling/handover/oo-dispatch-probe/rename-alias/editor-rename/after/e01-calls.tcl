proc zz {x} {return h:$x}
puts [zz 1]
proc user {} { return [zz 2] }
puts [user]
