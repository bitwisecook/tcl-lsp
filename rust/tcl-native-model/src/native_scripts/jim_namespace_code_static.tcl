
	if {[string first "::namespace inscope " $arg] == 0} {

		return $arg
	}
	list ::namespace inscope [uplevel 1 ::namespace current] $arg
