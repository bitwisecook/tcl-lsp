oo::class create B {}
oo::class create C { superclass B; method m {} {} }
set x [[C new] m]
