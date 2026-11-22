proc t {args} {return $args}
interp alias {} a1 {} t 1
interp alias {} a2 {} a1 2
puts [a2 3]
