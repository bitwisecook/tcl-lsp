# errorInfo / errorCode lifecycle.
puts "start ei=[info exists errorInfo] ec=[info exists errorCode]"
catch {error boom}
puts "after-error ei=[info exists errorInfo] ec=$errorCode"
puts [lindex [split $errorInfo \n] 0]
catch {expr {1/0}}
puts "divzero ec=$errorCode"
catch {set nosuch}
puts "lookup ec=$errorCode"
catch {error msg info MYCODE}
puts "explicit ec=$errorCode ei=[lindex [split $errorInfo \n] 0]"
proc p {} { catch {error inproc}; return [info exists errorCode] }
puts "proc-local=[p]"
