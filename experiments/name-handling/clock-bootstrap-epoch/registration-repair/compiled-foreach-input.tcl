proc boot {} {foreach cmd {add format scan} {proc ::tcl::clock::$cmd args {return deferred}}; set marker after}
boot
