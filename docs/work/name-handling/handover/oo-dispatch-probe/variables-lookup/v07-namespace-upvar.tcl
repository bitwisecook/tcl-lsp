namespace eval ::cfg { variable limit cfg }
proc p {} { namespace upvar ::cfg limit l; return $l ;# READ
}
puts [p]
