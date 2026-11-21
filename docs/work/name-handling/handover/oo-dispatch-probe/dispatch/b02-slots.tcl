oo::class create B { method b {} { return b } }
oo::class create C
oo::define C superclass -append B
puts [[C new] b]
oo::define C superclass -clear
puts [catch {[C new] b} m]:$m ;# BAD method gone
