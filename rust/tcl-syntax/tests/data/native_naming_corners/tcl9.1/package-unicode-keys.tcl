set code [catch {set one [format %c 233]; set two "e[format %c 769]"; package provide $one 1.0; package provide $two 2.0; list [package require $one] [package require $two]} result]
binary scan $result H* hex
puts [list $code $hex]
