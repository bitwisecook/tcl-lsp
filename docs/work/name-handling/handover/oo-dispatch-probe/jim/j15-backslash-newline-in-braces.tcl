# Tcl replaces backslash-newline-whitespace with one space even inside braces; Jim keeps it
set a {x\
  y}
puts [string length $a]
