# TIP 426: info cmdtype (9.0)
proc p {} {}
puts [catch {info cmdtype p} r]:$r
puts [catch {info cmdtype set} r]:$r
