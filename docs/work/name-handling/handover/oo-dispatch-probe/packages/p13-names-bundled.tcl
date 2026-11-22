puts [list [package require tcltest] [package require msgcat] [catch {package require http} m] [catch {package require nosuchpkg} m2] $m2]
