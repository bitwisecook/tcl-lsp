set target 1
set {$literal[tail]} 1
set code [catch {set observed {[info exists target]}} result]
puts [list inert_braces $code $result]
set code [catch {set observed "prefix[info exists {$literal[tail]}]suffix"} result]
puts [list quoted_child_literal_name $code $result]
set code [catch {expr {[info exists target]}} result]
puts [list expression_child $code $result]
set code [catch {set observed "[info totally_made_up]"} result]
puts [list unknown_actual_child $code $result]
