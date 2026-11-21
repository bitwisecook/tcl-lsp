oo::class create C { method m {} { set inner 1; my n }; method n {} { list [info level] [uplevel 1 {info exists inner}] } }
proc caller {} { [C new] m }
puts [caller]
