puts [list PATCHLEVEL [info patchlevel]]
puts [list EXECUTABLE [info nameofexecutable]]
set code [catch {info library} value]
puts [list LIBRARY $code $value]
