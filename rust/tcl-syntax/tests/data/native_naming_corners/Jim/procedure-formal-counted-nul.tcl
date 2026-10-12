set code [catch {set formal "x[format %c 0]tail"; proc p [list $formal] {list [info locals] [info exists x]}; p VALUE} result]
binary scan $result H* hex
puts [list $code $hex]
