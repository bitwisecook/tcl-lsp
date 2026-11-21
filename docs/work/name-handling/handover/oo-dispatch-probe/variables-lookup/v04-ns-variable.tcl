namespace eval ::a { variable x ns }
proc ::a::p {} { variable x; return $x ;# READ
}
puts [::a::p]
