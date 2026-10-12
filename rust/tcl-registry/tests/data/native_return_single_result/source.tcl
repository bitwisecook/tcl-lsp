proc f {v} {return $v}
foreach v {VALUE -code -level -bad -1} {set c [catch {f $v} r]; puts [list $v $c $r]}
