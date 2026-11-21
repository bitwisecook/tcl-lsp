package require Tk
entry .e -textvariable ::username
set ::username alice
puts [.e get]
proc build {} { set local bob; entry .e2 -textvariable local; return [.e2 get] }
puts "local: [build]" ;# BAD links the global "local", not the proc variable
checkbutton .c -variable ::flag
.c select
puts flag:$::flag
