set code [catch {set d {}; set local KEEP; dict update d absent local {}; list $d $local} result options]
puts $code
puts -nonewline $result
