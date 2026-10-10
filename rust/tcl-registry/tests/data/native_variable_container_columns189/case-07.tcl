puts [eval {set a {k OLD};set c [catch {array set a {other NEW}} r];list $c $r [catch {set a(k)} v] $v}]
