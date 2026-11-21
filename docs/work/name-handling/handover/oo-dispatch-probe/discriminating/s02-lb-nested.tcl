package require Tk
set lb [listbox .lb]
puts [catch {$lb insertt end d} m]
