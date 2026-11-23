# chan subcommand roster per release (error message lists them)
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t roster {chan nosuch}
t info {info commands chan}
