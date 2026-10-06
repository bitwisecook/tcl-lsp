
	set keys {}
	foreach {n v} $args {
		upvar $v var_$v
		if {[dict exists $varName $n]} {
			set var_$v [dict get $varName $n]
		}
	}
	catch {uplevel 1 $script} msg opts
	if {[info exists varName]} {
		foreach {n v} $args {
			if {[info exists var_$v]} {
				dict set varName $n [set var_$v]
			} else {
				dict unset varName $n
			}
		}
	}
	return {*}$opts $msg
