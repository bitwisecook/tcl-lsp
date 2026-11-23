# errorInfo and errorCode are rewritten by an error inside catch, whatever the script stored.
set errorInfo seeded
catch {error x}
puts [lindex [split $errorInfo \n] 0]
set errorCode seeded
catch {error y}
puts $errorCode
