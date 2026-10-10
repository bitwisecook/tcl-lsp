set log {}; proc member args {lappend ::log MEMBER}; trace add variable a(x) unset member; set before [list [array exists a] [info exists a(x)]]; unset a; list $before $log
