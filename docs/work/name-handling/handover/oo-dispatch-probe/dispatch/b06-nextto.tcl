oo::class create A { method m {} { return A } }
oo::class create B { method m {} { return B } }
oo::class create C { superclass A B; method m {} { return [nextto B] } }
puts [[C new] m]
