set argv "parent\u0000tail"
set argc 9
set jim::argv0 "\u00e9"
unset -nocomplain jim::lineedit
set child [interp]
puts [$child eval {list $argv [string length $argv] $argc $jim::argv0 [info exists jim::lineedit]}]
puts [string length $argv]
