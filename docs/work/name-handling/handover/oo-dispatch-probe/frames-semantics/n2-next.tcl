oo::class create B { method m {} { list [info level] [uplevel 1 {info exists callerv}] [uplevel 1 {info exists inner}] } }
oo::class create C { superclass B; method m {} { set inner 1; next } }
proc caller {} { set callerv 1; [C new] m }
puts [caller]
