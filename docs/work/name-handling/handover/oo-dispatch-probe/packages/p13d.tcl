set r [catch {package require msgcat} m o]
puts "r=$r m=$m"
puts [dict get $o -errorinfo]
puts [package ifneeded msgcat [lindex [package versions msgcat] end]]
puts done
