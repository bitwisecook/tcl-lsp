proc capture args {return $args}
puts [list vertical [capture ab]]
set args {one two}
set c [catch {list {a [inert]} "prefix[set x {]}]suffix" {*}$args} value]
puts [list nested $c $value]
set x puts
set c [catch {eval {[set x] actual}} value]
puts [list materialized-script $c $value]
set c [catch {eval {capture "a"b}} value]
puts [list welded-quote $c $value]
