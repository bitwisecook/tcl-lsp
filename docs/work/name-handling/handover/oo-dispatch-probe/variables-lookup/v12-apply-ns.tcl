namespace eval ::lib { variable x lib }
puts [apply {{} { variable x; return $x ;# READ
} ::lib}]
