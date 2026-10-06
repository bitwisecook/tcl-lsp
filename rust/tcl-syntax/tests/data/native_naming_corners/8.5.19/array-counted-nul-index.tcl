set code [catch {set key "k[format %c 0]tail"; set a($key) FULL; set a(k) SHORT; list [set a($key)] [set a(k)] [llength [array names a]]} result]
binary scan $result H* hex
puts [list $code $hex]
