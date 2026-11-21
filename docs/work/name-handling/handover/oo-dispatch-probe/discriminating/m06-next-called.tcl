oo::class create B { method a {} {} }
oo::class create C { superclass B; method a {} { next } }
[C new] a
