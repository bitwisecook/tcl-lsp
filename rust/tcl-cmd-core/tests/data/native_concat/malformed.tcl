set value [concat \{ foo]
puts [list $value [catch {llength $value} message] $message]
