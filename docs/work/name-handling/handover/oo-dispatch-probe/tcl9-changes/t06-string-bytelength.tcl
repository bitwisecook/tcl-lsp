# string bytelength: removed in 9.0
puts [catch {string bytelength [format %c 233]} r]:$r
puts [catch {string bytelength [format %c 0]} r]:$r
