proc t {a b} {return $a$b}
interp alias {} al {} t 1
puts [al 2] ;# CALL al
