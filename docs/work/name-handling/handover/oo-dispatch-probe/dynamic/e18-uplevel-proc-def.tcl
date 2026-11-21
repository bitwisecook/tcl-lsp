proc make {name} { uplevel #0 [list proc $name {} {return dyn}] }
make dyn1
puts [dyn1]
