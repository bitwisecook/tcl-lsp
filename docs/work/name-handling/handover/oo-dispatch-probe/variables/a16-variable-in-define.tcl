oo::class create C
oo::define C variable count
oo::define C method bump {} { incr count }
puts [[C new] bump]
