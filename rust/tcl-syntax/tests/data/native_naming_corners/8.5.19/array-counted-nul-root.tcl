set code [catch {set root "a[format %c 0]tail"; set ${root}(k) FULL; set a(k) SHORT; list [set ${root}(k)] [set a(k)]} result]
binary scan $result H* hex
puts [list $code $hex]
