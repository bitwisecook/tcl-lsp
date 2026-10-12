if {[info commands dict] eq ""} {return NOT_APPLICABLE}
unset -nocomplain d
set code [catch {dict unset d missing child} result]
set exists [info exists d]
set read [catch {set d} value]
list $code $result $exists $read $value
