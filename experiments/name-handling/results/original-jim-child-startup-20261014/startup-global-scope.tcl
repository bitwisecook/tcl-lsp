set argv GLOBAL
proc create_with_local {} {set argv LOCAL; interp}
set child [create_with_local]
puts [$child eval {set argv}]
puts $argv
