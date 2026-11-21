oo::class create A { method m {} { info level } }
oo::class create C { superclass A; method m {} { nextto A } }
proc caller {} { [C new] m }
puts [caller]
