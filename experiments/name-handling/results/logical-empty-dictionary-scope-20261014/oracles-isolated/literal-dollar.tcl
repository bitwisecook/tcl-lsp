puts [list VERSION [info patchlevel]]
set {$dict} {literal 12}
dict with {$dict} {}
puts [list RESULT [set {$dict}] [set literal] [info exists dict]]
