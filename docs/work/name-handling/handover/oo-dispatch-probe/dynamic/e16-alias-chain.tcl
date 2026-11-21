proc base {a b} { return $a$b }
interp alias {} one {} base x
interp alias {} two {} one
puts [two y]
puts [catch {two} m]:$m ;# BAD arity through two aliases
interp alias {} one {}
puts [catch {two y} m]:$m ;# BAD target alias deleted
