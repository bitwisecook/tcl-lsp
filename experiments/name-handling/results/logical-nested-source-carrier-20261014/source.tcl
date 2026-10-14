set ::hits 0
set ::other 0
set code [catch {set ::hits [incr ::other]}]
puts [list $code $::hits $::other]
