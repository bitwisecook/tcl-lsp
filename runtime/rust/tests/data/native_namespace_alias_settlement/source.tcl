proc ignore args {}
proc traced {name} {trace add variable v write ignore; variable $name X}
set v BEFORE
set code [catch {traced v} result]
puts [list traced $code $result $v]
set arr SCALAR
proc arrayDefine {name} {variable $name X}
set code [catch {arrayDefine arr(k)} result]
puts [list scalarArray $code $result [array exists arr] [info exists arr(k)] $arr]
proc replace {n1 n2 op} {uplevel #0 {unset v; set v REPLACED}}
set v BEFORE
trace add variable v write replace
proc rebound {name} {variable $name X; set $name}
set code [catch {rebound v} result]
puts [list rebound $code $result $v]
