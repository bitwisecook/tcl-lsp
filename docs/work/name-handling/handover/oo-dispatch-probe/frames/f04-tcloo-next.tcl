oo::class create B { method m {} { return B } }
oo::class create C { superclass B; method m {} { next ;# CALL
} }
puts [[C new] m]
