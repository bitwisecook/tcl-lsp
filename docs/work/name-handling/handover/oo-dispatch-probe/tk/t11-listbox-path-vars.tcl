package require Tk
set lb [listbox .lb]
$lb insert end a b c
$lb selection set 1
puts [$lb curselection]
puts [[listbox .lb2] size]
set w .lb
puts [$w get 0]
puts [catch {$lb insertt end d} m]:$m ;# BAD subcommand via variable
