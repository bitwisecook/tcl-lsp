set code [catch {set child [interp create]; proc target args {return [list PARENT $args]}; interp alias $child a {} target PREFIX; set result [interp eval $child {a ARG}]; interp delete $child; set result} result]
binary scan $result H* hex
puts [list $code $hex]
