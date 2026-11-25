set code [catch {set name "leaf[format %c 0]tail"; proc $name {} {return FULL}; list [eval [list $name]] [info commands leaf*]} result]
binary scan $result H* hex
puts [list $code $hex]
