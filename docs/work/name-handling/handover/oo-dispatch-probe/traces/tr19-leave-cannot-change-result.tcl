# A leave trace's return value does not replace the command result; an error does.
proc f {} { return 1 }
proc lv {cmd code res op} { return 99 }
trace add execution f leave lv
puts [f]
proc lv2 {cmd code res op} { error "leave failed" }
trace add execution f leave lv2
puts [catch {f} m]:$m
