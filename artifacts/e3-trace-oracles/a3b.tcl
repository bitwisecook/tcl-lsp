puts "A: [catch {uplevel 1 "list a \{ b"} m]:$m"
puts "B: [catch {uplevel 1 "set x \{"} m]:$m"
