puts [eval {array set a {k OLD};set c [catch {set copy $a} r];set a(k) NEW;list $c $r [info exists copy]}]
