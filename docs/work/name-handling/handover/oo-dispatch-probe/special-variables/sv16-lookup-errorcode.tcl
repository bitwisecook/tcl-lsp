# errorCode after failed variable reads, at top level, in a procedure and by computed name.
catch {set nosuch}; puts "1 $errorCode"
catch {puts $nosuch}; puts "2 $errorCode"
proc p {} { catch {set nosuch}; puts "3 $::errorCode"; catch {puts $nosuch}; puts "4 $::errorCode" }
p
set s nosuch; catch {set $s}; puts "5 $errorCode"
catch {set a(1)}; puts "6 $errorCode"
catch {unset nosuch}; puts "7 $errorCode"
