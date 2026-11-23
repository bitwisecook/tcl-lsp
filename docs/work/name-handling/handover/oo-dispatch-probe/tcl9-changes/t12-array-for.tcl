array set a {k 1}
puts [catch {array for {k v} a {puts "$k=$v"}} r]:$r
