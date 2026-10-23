set quoted "﻿42"
set bare ﻿42
puts [list [string equal $quoted $bare] [string length $quoted] [scan [string index $quoted 0] %c] [string range $quoted 1 end]]
