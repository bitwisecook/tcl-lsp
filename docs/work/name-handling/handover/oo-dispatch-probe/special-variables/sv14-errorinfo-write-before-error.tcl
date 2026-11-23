# Writing errorInfo before 'error' with no info argument.
set errorInfo seeded
catch {error x}
puts [lindex [split $errorInfo \n] 0]
set errorCode seeded
catch {error y}
puts $errorCode
