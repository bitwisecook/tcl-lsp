oo::class create C { method m {} { list [info level] [uplevel 1 {info exists callerv}] } }
proc caller {} { set callerv 1; [C new] m }
puts [caller]
