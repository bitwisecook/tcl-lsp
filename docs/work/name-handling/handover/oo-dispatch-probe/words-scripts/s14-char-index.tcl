set s "a\U0001F600b"
puts [list [string length $s] [scan [string index $s 1] %c] [string range $s 2 end] [string length [encoding convertto utf-8 $s]]]
