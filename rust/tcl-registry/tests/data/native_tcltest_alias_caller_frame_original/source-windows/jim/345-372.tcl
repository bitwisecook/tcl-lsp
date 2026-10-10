	if {[catch {uplevel 1 $a(-setup)} msg]} {
		if {$::testinfo(verbose)} {
			puts "-setup failed: $msg"
		}
	}
	set rc [catch {uplevel 1 $a(-body)} result opts]
	if {[catch {uplevel 1 $a(-cleanup)} msg]} {
		if {$::testinfo(verbose)} {
			puts "-cleanup failed: $msg"
		}
	}

	if {[info return $rc] ni $a(-returnCodes) && $rc ni $a(-returnCodes)} {
		set ok 0
		set expected "rc=[list $a(-returnCodes)] result=[list $a(-result)]"
		set actual "rc=[info return $rc] result=[list $result]"
		# Now for the template, update -returnCodes
		set a(-returnCodes) [info return $rc]
	} else {
		if {$a(-match) eq "exact"} {
			set ok [string equal $a(-result) $result]
		} elseif {$a(-match) eq "glob"} {
			set ok [string match $a(-result) $result]
		} elseif {$a(-match) eq "regexp"} {
			set ok [regexp $a(-result) $result]
		} else {
			return -code error "$id: unknown match type: $a(-match)"
		}
