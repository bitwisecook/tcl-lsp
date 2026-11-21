proc f {} { return f }
puts [f]
rename f {}
puts [catch f m]:$m ;# BAD deleted
