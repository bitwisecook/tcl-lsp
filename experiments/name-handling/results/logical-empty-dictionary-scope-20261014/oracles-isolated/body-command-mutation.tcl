puts [list VERSION [info patchlevel]]
set d {body_value 15}
dict with d {proc newly_defined_name {} {}}
puts [list RESULT [info commands newly_defined_name] [set body_value]]
