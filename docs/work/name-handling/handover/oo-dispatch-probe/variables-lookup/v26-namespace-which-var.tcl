namespace eval ::a { variable v which }
proc ::a::p {} { return [namespace which -variable v] ;# READ
}
puts [::a::p]
