proc target {args} {return t:$args}
oo::class create C { forward f target a }
puts [[C new] f b] ;# CALL f
