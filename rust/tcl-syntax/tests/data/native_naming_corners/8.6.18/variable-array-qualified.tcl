set code [catch {namespace eval ::n {variable a; set a(k) VALUE}; proc p {} {variable ::n::a; set a(k)}; p} result]
binary scan $result H* hex
puts [list $code $hex]
