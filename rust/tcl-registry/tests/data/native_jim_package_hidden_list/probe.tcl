puts "VERSION|[info patchlevel]"
foreach op {list names __r2286_absent__} {set code [catch {package $op} result]; puts [list RESULT $op $code $result]}
