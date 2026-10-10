set log {}; proc member args {lappend ::log MEMBER}; proc owner {} {trace add variable a(x) unset member; list [array exists a] [info exists a(x)]}; set before [owner]; list $before $log
