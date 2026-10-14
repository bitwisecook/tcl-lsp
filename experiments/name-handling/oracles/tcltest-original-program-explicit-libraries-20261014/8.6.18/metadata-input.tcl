puts [list [info patchlevel] [info library] [package require tcltest]]
puts [package ifneeded tcltest [package present tcltest]]
