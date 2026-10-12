
	upvar $varName dict
	if {[exists dict] && [dict exists $dict $key]} {
		set list [dict get $dict $key]
	}
	lappend list {*}$value
	dict set dict $key $list
