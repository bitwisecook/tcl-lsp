
	foreach name $args {
		if {$name ni {:: ""}} {
			set name [uplevel 1 [list ::namespace canon $name]]
			foreach i [info commands ${name}::*] { rename $i "" }
			uplevel #0 [list unset {*}[info globals ${name}::*]]
		}
	}
