package require Tk
ttk::style configure Big.TButton -padding 10
ttk::button .b -text Go -style Big.TButton
ttk::combobox .c -values {a b c} -state readonly
.c current 1
puts [.c get]
puts [catch {ttk::button .b2 -bg red} m]:$m ;# BAD ttk has no -bg
puts [catch {.c current 9} m]:$m ;# BAD index
