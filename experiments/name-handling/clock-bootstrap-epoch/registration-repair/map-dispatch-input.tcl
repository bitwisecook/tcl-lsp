foreach cmd {add format scan} {proc ::tcl::clock::$cmd args {return deferred}}
puts [clock ad]
puts [clock for]
puts [clock sc]
