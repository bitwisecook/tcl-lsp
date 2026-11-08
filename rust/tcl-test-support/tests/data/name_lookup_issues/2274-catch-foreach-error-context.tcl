catch {foreach x 1 {error direct EI}}
puts [list $::errorInfo]
proc p {} {catch {foreach x 1 {error direct EI}}; set ::errorInfo}
puts [list [p]]
