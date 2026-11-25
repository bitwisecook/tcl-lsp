set code [catch {set ::n VALUE; proc p {} {global ::n; list [info locals] [info exists n] [set ::n]}; p} result]
binary scan $result H* hex
puts [list $code $hex]
