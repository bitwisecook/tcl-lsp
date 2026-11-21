array set a {k array}
proc p {} { upvar 1 a arr; return $arr(k) ;# READ
}
puts [p]
