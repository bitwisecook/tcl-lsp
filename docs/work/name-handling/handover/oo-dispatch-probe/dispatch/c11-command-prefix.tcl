proc show {prefix value} { return $prefix$value }
set cmd [list show >]
puts [{*}$cmd 1]
puts [catch {{*}$cmd} m]:$m ;# BAD arity
