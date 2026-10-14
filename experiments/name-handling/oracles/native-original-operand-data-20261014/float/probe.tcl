set n 5
set code [catch {incr n 1.5} value]
puts [list float-amount $code $n $value]
