const limit 3
puts $limit
puts [catch {set limit 4} m]:$m ;# BAD const
