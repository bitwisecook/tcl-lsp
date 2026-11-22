proc helper {x} {return h:$x}
puts [helper 1]
proc user {} { return [helper 2] }
puts [user]
