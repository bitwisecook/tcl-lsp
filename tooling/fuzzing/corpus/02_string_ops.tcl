# String operations
set s "hello world"
set len [string length $s]
set upper [string toupper $s]
set first [string index $s 0]
set sub [string range $s 0 4]
set trimmed [string trim "  spaces  "]
set mapped [string map {h H w W} $s]
set pos [string first "world" $s]
