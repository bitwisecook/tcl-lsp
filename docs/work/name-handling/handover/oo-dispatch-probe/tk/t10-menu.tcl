package require Tk
menu .m -tearoff 0
menu .m.file -tearoff 0
.m add cascade -label File -menu .m.file
.m.file add command -label Quit -command {puts quit}
. configure -menu .m
.m.file invoke 0
puts [catch {.m add item -label x} m]:$m ;# BAD entry type
puts [catch {.m.file entryconfigure 0 -lable y} m]:$m ;# BAD option
