set code [catch {set log {}; proc observe {old new op} {lappend ::log [list $old $new $op]}; proc leaf {} {return VALUE}; trace add command leaf rename observe; rename leaf moved; list [moved] $log} result]
binary scan $result H* hex
puts [list $code $hex]
