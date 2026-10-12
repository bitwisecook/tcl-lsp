set a value
set code [catch {array names a -bad x} result]
set error_code {}
if {$code != 0 && [info exists ::errorCode]} {set error_code $::errorCode}
puts [list $code $result $error_code]
