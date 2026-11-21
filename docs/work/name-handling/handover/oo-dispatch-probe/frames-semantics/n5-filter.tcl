oo::class create C { method m {} { info level }; method f args { next {*}$args }; filter f }
proc caller {} { [C new] m }
puts [caller]
