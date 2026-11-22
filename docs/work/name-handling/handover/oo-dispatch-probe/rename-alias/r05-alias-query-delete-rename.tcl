proc t {a b} {return $a$b}
interp alias {} al {} t 1
puts [list [interp alias {} al] [lsearch -exact [interp aliases] al]]
rename al al2
puts [list [al2 2] [lsearch -exact [interp aliases] al2] [lsearch -exact [interp aliases] al]]
interp alias {} al2 {}
puts [list [catch {al2 3} m] [lsearch -exact [interp aliases] al2]]
