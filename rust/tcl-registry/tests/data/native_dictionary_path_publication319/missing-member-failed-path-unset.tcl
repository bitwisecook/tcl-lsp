if {[info commands dict] eq ""} {return NOT_APPLICABLE}
set outer {keep OLD}
set code [catch {dict unset outer(member) missing child} result]
set exists [info exists outer(member)]
set read [catch {set outer(member)} value]
list $code $result $exists $read $value [array get outer]
