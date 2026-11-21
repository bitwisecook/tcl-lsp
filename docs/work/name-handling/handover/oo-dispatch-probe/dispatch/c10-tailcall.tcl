proc a {x} { tailcall b $x }
proc b {x} { return b:$x }
puts [a 1]
proc c {} { tailcall b }
puts [catch c m]:$m ;# BAD arity via tailcall
