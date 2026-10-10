puts [eval {set a {k OLD};set c [catch {set a(k) NEW} r];list $c $r [array exists a]}]
