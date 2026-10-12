puts [eval {proc p {} {array set a {k OLD};upvar 0 a root;set c [catch {set root NEW} r];list $c $r [array exists a]};p}]
