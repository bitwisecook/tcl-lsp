proc capture {args} {return $args}
rename capture {}
puts [list missing_after_delete [catch {{} {}} result] $result]
