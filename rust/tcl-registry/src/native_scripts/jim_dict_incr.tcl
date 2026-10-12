
	upvar $varName dict
	if {[exists dict] && [dict exists $dict $key]} {
		set value [dict get $dict $key]
	}
	incr value $increment
	dict set dict $key $value
