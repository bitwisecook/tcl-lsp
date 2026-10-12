set code [catch {set child [interp]; $child alias a list PREFIX; set result [$child eval {a ARG}]; $child delete; set result} result]
binary scan $result H* hex
puts [list $code $hex]
