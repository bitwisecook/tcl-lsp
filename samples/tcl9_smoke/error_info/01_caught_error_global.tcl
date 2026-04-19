# After catch, ::errorInfo and ::errorCode are populated
catch { error boom } msg
puts $msg
puts $::errorInfo
puts $::errorCode
