oo::class create B { constructor {} { set ::lv [info level] } }
oo::class create C { superclass B; constructor {} { next } }
proc caller {} { C new; return $::lv }
puts [caller]
