oo::class create C { method a {} { my b ;# CALL
}; method b {} { return C.b } }
puts [[C new] a]
