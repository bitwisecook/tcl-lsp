proc literal {} {set v LOCAL; variable v GLOBAL; return NEVER}
proc dynamic {name} {set v LOCAL; variable $name GLOBAL; return NEVER}
proc arrayDefine {name} {variable $name X}
set v BEFORE
set code [catch {literal} result]
puts [list literal $code $result $v]
set v BEFORE
set code [catch {dynamic v} result]
puts [list dynamic $code $result $v]
set code [catch {arrayDefine arr(k)} result]
puts [list arrayDefine $code $result [array exists arr] [info exists arr(k)]]
proc prefix {name} {variable v 1 $name 2}
set code [catch {prefix a)} result]
puts [list prefix $code $result $v]
proc normal {name} {variable $name X; set $name Y}
set code [catch {normal v} result]
puts [list normal $code $result $v]
