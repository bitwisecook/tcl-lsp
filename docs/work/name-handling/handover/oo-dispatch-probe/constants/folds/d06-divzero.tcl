proc p {} { if {[catch {expr {1/0}} m]} { return "err $m" }; return ok }
puts [p]
