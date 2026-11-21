package require Tk
button .b; pack .b
update idletasks
puts [winfo exists .b][winfo ismapped .b]
puts [winfo reqwidth .b]
