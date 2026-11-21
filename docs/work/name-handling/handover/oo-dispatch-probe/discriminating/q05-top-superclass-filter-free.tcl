oo::class create B { method f {} {} }
oo::class create C { superclass B; method m {} {} }
[C new] f
