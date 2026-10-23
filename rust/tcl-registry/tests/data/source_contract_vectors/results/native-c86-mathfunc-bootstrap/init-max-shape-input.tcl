namespace eval ::tcl::mathfunc {
	proc max {args} {
	    if {![llength $args]} {
		return -code error \
		    "not enough arguments to math function \"max\""
	    }
	    set val -Inf
	    foreach arg $args {
		# This will handle forcing the numeric value without
		# ruining the internal type of a numeric object
		if {[catch {expr {double($arg)}} err]} {
		    return -code error $err
		}
		if {$arg > $val} {set val $arg}
	    }
	    return $val
	}
}
set marker after
puts $marker
