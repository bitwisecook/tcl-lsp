set code [catch {oo::class create C {variable x; constructor {} {set x VALUE}; method get {} {return $x}}; C create object; object get} result]
binary scan $result H* hex
puts [list $code $hex]
