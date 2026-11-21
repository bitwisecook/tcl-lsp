proc greet {greeting name} { return "$greeting $name" }
interp alias {} hello {} greet Hello
puts [hello World]
puts [catch {hello} m]:$m ;# BAD arity
