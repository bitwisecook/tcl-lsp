proc capture {args} {return $args}
set object capture
set {$literal} capture
set array(key) capture
proc self {args} {return capture}
puts [list variable [$object method]]
puts [list literal [${$literal} {with space}]]
puts [list indexed [$array(key) method]]
puts [list command [[self object] method]]
rename capture {object name}
puts [list grouped [{object name} "with space"]]
rename {object name} capture
puts [list quoted ["$object" method]]
proc {} {args} {return $args}
puts [list empty [{} {}]]
set {} {}
puts [list empty_variable [${} {}]]
