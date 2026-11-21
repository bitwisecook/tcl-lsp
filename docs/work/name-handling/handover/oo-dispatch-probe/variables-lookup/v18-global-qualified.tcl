namespace eval ::ns { variable v nsglobal }
proc p {} { global ::ns::v; return $v ;# READ
}
puts [p]
