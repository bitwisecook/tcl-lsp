puts [list provider [info patchlevel]]
set entered 0
proc count args {incr ::entered}
proc auto_import pattern {return}
trace add execution auto_import enter count
catch {namespace import ::missing::*} result
puts [list $entered $result]
