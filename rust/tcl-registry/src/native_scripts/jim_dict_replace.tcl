
	if {[llength ${key value}] % 2} {
		tailcall {dict replace}
	}
	tailcall dict merge $dictionary ${key value}
