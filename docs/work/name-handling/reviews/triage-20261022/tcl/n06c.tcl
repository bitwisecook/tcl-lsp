oo::class create A {method m {} {return A}}
oo::class create B {method m {} {return B}}
puts [[A new] m]
puts [[B new] m]
