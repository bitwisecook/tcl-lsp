proc tick {n} {}
puts [catch {after 10 {tick}} m]
