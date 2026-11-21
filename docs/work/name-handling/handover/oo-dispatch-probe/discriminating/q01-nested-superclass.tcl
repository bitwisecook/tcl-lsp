oo::class create B {}
oo::class create C { superclass B; method m {} {} }
puts [[C new] m]
