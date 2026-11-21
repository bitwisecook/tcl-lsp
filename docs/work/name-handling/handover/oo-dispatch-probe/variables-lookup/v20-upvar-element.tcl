array set a {k elem}
proc p {} { upvar 1 a(k) e; return $e ;# READ
}
puts [p]
