set code [catch {oo::class create C {method get {} {return VALUE}}; set name "object[format %c 0]tail"; C create $name; eval [list $name get]} result]
binary scan $result H* hex
puts [list $code $hex]
