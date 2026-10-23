
	set autoprefix "$command "
	set badopts "should be \"ensemble command ?-automap prefix?\""
	if {[llength $args] % 2 != 0} {
		return -code error "wrong # args: $badopts"
	}
	foreach {opt value} $args {
		switch -- $opt {
			-automap { set autoprefix $value }
			default { return -code error "wrong # args: $badopts" }
		}
	}
	proc $command {subcmd args} {autoprefix {mapping {}}} {
		if {![dict exists $mapping $subcmd]} {

			if {$subcmd in {-commands -help}} {

				set prefixlen [string length $autoprefix]
				set subcmds [lmap p [lsort [info commands -all $autoprefix*]] {
					string range $p $prefixlen end
				}]
				if {$subcmd eq "-commands"} {
					return $subcmds
				}
				set command [lindex [info level 0] 0]
				return "Usage: \"$command command ... \", where command is one of: [join $subcmds ", "]"
			}

			dict set mapping $subcmd ${autoprefix}$subcmd
		}

		tailcall [dict get $mapping $subcmd] {*}$args
	}
