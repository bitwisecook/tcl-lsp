set ::count 0
proc bump {} { incr ::count; return $::count }
proc p {} { return [list [bump] [bump]] }
puts [p]
