set code [catch {set name "target[format %c 0]tail"; set $name FULL; set target SHORT; proc p {name} {upvar #0 $name alias; set alias}; p $name} result]
binary scan $result H* hex
puts [list $code $hex]
