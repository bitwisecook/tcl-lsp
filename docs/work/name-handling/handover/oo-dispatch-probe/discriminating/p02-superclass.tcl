oo::class create B {}
oo::class create C { superclass B; method m {} {} }
[C new] m
