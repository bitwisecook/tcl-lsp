foreach name {argv argc argv0 jim::argv0 jim::exe jim::lineedit} {unset -nocomplain $name}
set child [interp]
puts [$child eval {list [info exists argv] [info exists argc] [info exists argv0] [info exists jim::argv0] [info exists jim::exe] [info exists jim::lineedit]}]
