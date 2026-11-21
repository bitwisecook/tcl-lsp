oo::class create C { method m {} {}; method f args { next {*}$args }; filter f }
[C new] m
