proc define {} { uplevel 1 {set made 1} }
proc use {} { define; return $made }
puts [use]
proc bad {} { uplevel 1 {set a [llength]} } ;# BAD arity inside uplevel
puts [catch bad m]:$m
