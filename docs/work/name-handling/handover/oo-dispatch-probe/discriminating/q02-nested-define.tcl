oo::class create C { method m {} {} }
oo::define C method n {} {}
puts [[C new] m]
