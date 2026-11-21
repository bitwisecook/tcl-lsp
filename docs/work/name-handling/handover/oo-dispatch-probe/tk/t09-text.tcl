package require Tk
text .t
.t insert end "hello\nworld" greet
.t tag configure greet -foreground blue
puts [.t get 1.0 "end - 1c"]
puts [.t index "insert linestart"]
.t mark set here 2.0
puts [catch {.t get 1.0 endd} m]:$m ;# BAD index
puts [catch {.t tag configure greet -colour red} m]:$m ;# BAD tag option
