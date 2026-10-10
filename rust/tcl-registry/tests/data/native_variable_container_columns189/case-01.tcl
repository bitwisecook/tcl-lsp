puts [eval {array set a {k OLD};set c [catch {set a NEW} r];list $c $r [array exists a]}]
