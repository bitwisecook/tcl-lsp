proc p {} { catch {set c caught}; regexp {(\w+)} word -> m; return $c$m ;# READ
}
puts [p]
