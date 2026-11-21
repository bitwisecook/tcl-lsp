proc define {} { uplevel 1 {set made uplevel} }
proc p {} { define; return $made ;# READ
}
puts [p]
