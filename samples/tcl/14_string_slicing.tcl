# String slicing: indices count characters from 0, and `end` names the last
# one, so `end-4` is five characters from the end. An index past either end
# is clamped rather than an error.
set greeting "Hello, World"
puts [string range $greeting 0 4]
puts [string range $greeting 7 end]
puts [string range $greeting end-4 end]
puts [string range $greeting 5 100]
