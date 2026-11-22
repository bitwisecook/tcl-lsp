puts [list [scan "\x41" %c] [scan "\101" %c] [scan "é" %c] [string length "\x414"] [scan [string index "\x414" 0] %c] [scan "\U0001F600" %c]]
