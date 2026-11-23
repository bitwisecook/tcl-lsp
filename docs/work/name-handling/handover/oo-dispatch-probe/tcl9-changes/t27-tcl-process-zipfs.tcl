puts [catch {tcl::process list} r]:[expr {$r eq "" ? "empty" : "list"}]
puts [catch {zipfs root} r]:$r
