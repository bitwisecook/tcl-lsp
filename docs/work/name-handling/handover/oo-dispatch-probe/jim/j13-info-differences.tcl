# Jim info differences
proc p {} {{s 7}} {return [info statics p]}
puts [catch {p} r]:$r
puts [catch {info level} r]:$r
puts [catch {info frame} r]:[string is integer $r]
alias e info exists
puts [catch {info alias e} r]:$r
puts [catch {info references} r]:[llength $r]
puts [catch {info source {puts}} r]:[llength $r]
puts [catch {info returncodes} r]:[llength $r]
