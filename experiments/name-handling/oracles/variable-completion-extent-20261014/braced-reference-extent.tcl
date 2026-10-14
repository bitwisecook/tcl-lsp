set [format "a%cb" 123] FIRST
set [format "a%cb%cc" 123 125] NEST
set code [catch {list ${a{b}c}} value]
puts [list braced-variable code $code value $value]
set array(key) ELEMENT
set code [catch {list $array(key)suffix} value]
puts [list array-suffix code $code value $value]
