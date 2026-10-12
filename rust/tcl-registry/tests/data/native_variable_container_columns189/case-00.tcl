puts [eval {array set a {k OLD};set c [catch {set a} r];list $c $r}]
