set code [catch {set name "v[format %c 0]tail"; set $name FULL; set v SHORT; proc p {name} {global $name; list [info locals] [set $name]}; p $name} result]
binary scan $result H* hex
puts [list $code $hex]
