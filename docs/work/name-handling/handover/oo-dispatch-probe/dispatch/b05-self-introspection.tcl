oo::class create B { method m {} { return [list [self next] [self method]] } }
oo::class create C { superclass B; method m {} { return [list [self call] [next]] } }
puts [llength [[C new] m]]
