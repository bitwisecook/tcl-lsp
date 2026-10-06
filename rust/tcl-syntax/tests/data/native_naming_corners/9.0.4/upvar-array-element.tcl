set code [catch {set a(k) OLD; proc p {} {upvar #0 a(k) alias; set alias NEW; list [set ::a(k)] [info exists alias]}; p} result]
binary scan $result H* hex
puts [list $code $hex]
