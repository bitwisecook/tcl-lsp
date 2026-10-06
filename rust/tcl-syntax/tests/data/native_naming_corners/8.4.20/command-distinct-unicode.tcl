set code [catch {set one [format %c 233]; set two "e[format %c 769]"; proc $one {} {return ONE}; proc $two {} {return TWO}; list [eval [list $one]] [eval [list $two]]} result]
binary scan $result H* hex
puts [list $code $hex]
