package require Tk
puts [catch {tk_messageBox -type okk -message hi} m]:$m ;# BAD type
puts [catch {tk_getOpenFile -filetypess {}} m]:$m ;# BAD option
