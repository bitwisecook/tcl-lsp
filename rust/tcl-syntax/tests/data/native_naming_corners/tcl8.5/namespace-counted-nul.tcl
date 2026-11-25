set code [catch {set name "::n[format %c 0]tail"; namespace eval $name {namespace current}} result]
binary scan $result H* hex
puts [list $code $hex]
