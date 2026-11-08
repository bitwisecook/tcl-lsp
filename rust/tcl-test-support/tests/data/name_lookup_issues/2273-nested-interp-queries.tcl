interp create a
a eval {interp create -safe b}
puts [interp exists {a b}]
puts [catch {interp issafe {a b}} msg]
puts $msg
