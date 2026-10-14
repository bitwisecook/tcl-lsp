puts [list VERSION [info patchlevel]]
set d {odd}
set code [catch {dict with d {}} result]
puts [list RESULT $code $result]
