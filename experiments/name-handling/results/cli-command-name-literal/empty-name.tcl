puts [list provider [info patchlevel]]
set name {}
puts [list before [namespace which -command $name]]
proc $name {} {return empty-target}
puts [list after [namespace which -command $name]]
set code [catch [list $name] value]
puts [list invoke $code $value]
