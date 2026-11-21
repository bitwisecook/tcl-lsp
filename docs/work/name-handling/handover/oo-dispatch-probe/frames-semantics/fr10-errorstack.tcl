proc inner {} { error boom }
proc outer {} { inner }
catch {outer} m o
puts [dict get $o -errorstack]
puts [llength [split [dict get $o -errorinfo] \n]]
