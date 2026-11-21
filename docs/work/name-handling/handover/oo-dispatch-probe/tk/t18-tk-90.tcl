package require Tk
puts [tk windowingsystem]
puts [catch {tk sysnotify Title Body} m]
puts [catch {tk print} m]:$m ;# BAD needs a window
