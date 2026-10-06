set code [catch {set ::target ROOT; proc p {} {upvar #0 ::target :alias; list ${:alias} [info locals]}; p} result]
binary scan $result H* hex
puts [list $code $hex]
