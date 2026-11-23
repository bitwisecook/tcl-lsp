# std channels, names, and option key lists
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t names {lsort [file channels]}
t chan-names {lsort [chan names]}
t stdout-keys {set r {}; foreach {k v} [fconfigure stdout] {lappend r $k}; set r}
t stdin-keys {set r {}; foreach {k v} [fconfigure stdin] {lappend r $k}; set r}
t stderr-buffering {fconfigure stderr -buffering}
t stdout-buffering {fconfigure stdout -buffering}
